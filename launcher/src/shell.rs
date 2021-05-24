use std::mem::ManuallyDrop;

use iced_graphics::window;
use iced_winit::{
    application::{build_user_interface, requests_exit, update, State},
    conversion,
    executor::Executor,
    futures::{self as futures, channel::mpsc},
    mouse,
    winit::{self, window::UserAttentionType},
    Cache, Clipboard, Debug, Error, Proxy, Runtime, Settings,
};

mod iced_futures {
    pub use iced_winit::futures;
}

pub trait Application: iced_winit::Application {
    fn is_visible(&self) -> bool;
}

pub fn run<A, E, C>(
    settings: Settings<A::Flags>,
    compositor_settings: C::Settings,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: window::Compositor<Renderer = A::Renderer> + 'static,
{
    use futures::task;
    use futures::Future;
    use winit::event_loop::EventLoop;

    let mut debug = Debug::new();
    debug.startup_started();

    let (compositor, renderer) = C::new(compositor_settings)?;

    let event_loop = EventLoop::with_user_event();

    let mut runtime = {
        let proxy = Proxy::new(event_loop.create_proxy());
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, init_command) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    let subscription = application.subscription();

    runtime.spawn(init_command);
    runtime.track(subscription);

    let window = settings
        .window
        .into_builder(
            &application.title(),
            application.mode(),
            event_loop.primary_monitor(),
        )
        .build(&event_loop)
        .map_err(Error::WindowCreationFailed)?;

    let (mut sender, receiver) = mpsc::unbounded();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        debug,
        receiver,
        window,
        settings.exit_on_close_request,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;

        if let ControlFlow::Exit = control_flow {
            return;
        }

        let event = match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. },
                window_id,
            } => Some(winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(*new_inner_size),
                window_id,
            }),
            _ => event.to_static(),
        };

        if let Some(event) = event {
            sender.start_send(event).expect("Send event");

            let poll = instance.as_mut().poll(&mut context);

            *control_flow = match poll {
                task::Poll::Pending => ControlFlow::Wait,
                task::Poll::Ready(_) => ControlFlow::Exit,
            };
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, Proxy<A::Message>, A::Message>,
    mut debug: Debug,
    mut receiver: mpsc::UnboundedReceiver<winit::event::Event<'_, A::Message>>,
    window: winit::window::Window,
    exit_on_close_request: bool,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: window::Compositor<Renderer = A::Renderer> + 'static,
{
    use iced_futures::futures::stream::StreamExt;
    use winit::event;

    let surface = compositor.create_surface(&window);
    let mut clipboard = Clipboard::connect(&window);

    let mut visible = true;

    let mut state = State::new(&application, &window);
    let mut viewport_version = state.viewport_version();
    let mut swap_chain = {
        let physical_size = state.physical_size();

        compositor.create_swap_chain(&surface, physical_size.width, physical_size.height)
    };

    let mut user_interface = ManuallyDrop::new(build_user_interface(
        &mut application,
        Cache::default(),
        &mut renderer,
        state.logical_size(),
        &mut debug,
    ));

    let mut primitive = user_interface.draw(&mut renderer, state.cursor_position());
    let mut mouse_interaction = mouse::Interaction::default();

    let mut events = Vec::new();
    let mut messages = Vec::new();

    debug.startup_finished();

    while let Some(event) = receiver.next().await {
        match event {
            event::Event::MainEventsCleared => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }

                debug.event_processing_started();

                let statuses = user_interface.update(
                    &events,
                    state.cursor_position(),
                    &renderer,
                    &mut clipboard,
                    &mut messages,
                );

                debug.event_processing_finished();

                for event in events.drain(..).zip(statuses.into_iter()) {
                    runtime.broadcast(event);
                }

                if !messages.is_empty() {
                    let cache = ManuallyDrop::into_inner(user_interface).into_cache();

                    // Update application
                    update(
                        &mut application,
                        &mut runtime,
                        &mut debug,
                        &mut clipboard,
                        &mut messages,
                    );

                    // Update window
                    state.synchronize(&application, &window);

                    if visible != application.is_visible() {
                        visible = !visible;
                        window.set_visible(visible);
                        if visible {
                            window.request_user_attention(Some(UserAttentionType::Informational));
                        }
                    }

                    let should_exit = application.should_exit();

                    user_interface = ManuallyDrop::new(build_user_interface(
                        &mut application,
                        cache,
                        &mut renderer,
                        state.logical_size(),
                        &mut debug,
                    ));

                    if should_exit {
                        break;
                    }
                }

                debug.draw_started();
                primitive = user_interface.draw(&mut renderer, state.cursor_position());
                debug.draw_finished();

                window.request_redraw();
            }
            event::Event::UserEvent(message) => {
                messages.push(message);
            }
            event::Event::RedrawRequested(_) => {
                let physical_size = state.physical_size();

                if physical_size.width == 0 || physical_size.height == 0 {
                    continue;
                }

                debug.render_started();
                let current_viewport_version = state.viewport_version();

                if viewport_version != current_viewport_version {
                    let logical_size = state.logical_size();

                    debug.layout_started();
                    user_interface = ManuallyDrop::new(
                        ManuallyDrop::into_inner(user_interface)
                            .relayout(logical_size, &mut renderer),
                    );
                    debug.layout_finished();

                    debug.draw_started();
                    primitive = user_interface.draw(&mut renderer, state.cursor_position());
                    debug.draw_finished();

                    swap_chain = compositor.create_swap_chain(
                        &surface,
                        physical_size.width,
                        physical_size.height,
                    );

                    viewport_version = current_viewport_version;
                }

                let new_mouse_interaction = compositor.draw(
                    &mut renderer,
                    &mut swap_chain,
                    state.viewport(),
                    state.background_color(),
                    &primitive,
                    &debug.overlay(),
                );

                debug.render_finished();

                if new_mouse_interaction != mouse_interaction {
                    window.set_cursor_icon(conversion::mouse_interaction(new_mouse_interaction));

                    mouse_interaction = new_mouse_interaction;
                }

                // TODO: Handle animations!
                // Maybe we can use `ControlFlow::WaitUntil` for this.
            }
            event::Event::WindowEvent {
                event: window_event,
                ..
            } => {
                if requests_exit(&window_event, state.modifiers()) && exit_on_close_request {
                    break;
                }

                state.update(&window, &window_event, &mut debug);

                if let Some(event) =
                    conversion::window_event(&window_event, state.scale_factor(), state.modifiers())
                {
                    events.push(event);
                }
            }
            _ => {}
        }
    }

    // Manually drop the user interface
    drop(ManuallyDrop::into_inner(user_interface));
}
