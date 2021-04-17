use std::{ops::Deref, sync::Arc};

use ash::{
    prelude::VkResult,
    version::DeviceV1_0,
    vk::{self, FramebufferCreateFlags, FramebufferCreateInfo, GraphicsPipelineCreateInfo},
};

use super::{DerefHandle, Device, Handle, HandleWrapper};

pub struct Object<T: Handle> {
    pub(super) device: Arc<Device>,
    pub(super) handle: T,
}

impl<T: Handle> HandleWrapper for Object<T> {
    type Handle = T;
}

impl<T: Handle> Drop for Object<T> {
    fn drop(&mut self) {
        self.device.destroy_handle(self.handle)
    }
}

impl<T: DerefHandle> Deref for Object<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

pub type Fence = Object<vk::Fence>;
pub type Semaphore = Object<vk::Semaphore>;
pub type ImageView = Object<vk::ImageView>;
pub type RenderPass = Object<vk::RenderPass>;
pub type ShaderModule = Object<vk::ShaderModule>;
pub type PipelineCache = Object<vk::PipelineCache>;
pub type Pipeline = Object<vk::Pipeline>;
pub type PipelineLayout = Object<vk::PipelineLayout>;
pub type Framebuffer = Object<vk::Framebuffer>;
pub type Sampler = Object<vk::Sampler>;

impl Fence {
    pub fn wait(&self) -> VkResult<()> {
        unsafe {
            self.device
                .inner
                .wait_for_fences(&[self.handle], false, u64::MAX)
        }
    }

    pub fn reset(&self) -> VkResult<()> {
        unsafe { self.device.inner.reset_fences(&[self.handle]) }
    }

    pub fn status(&self) -> VkResult<bool> {
        unsafe { self.device.inner.get_fence_status(self.handle) }
    }
}

impl PipelineCache {
    pub fn create_graphics(
        &self,
        create_infos: &[GraphicsPipelineCreateInfo],
    ) -> VkResult<Vec<Pipeline>> {
        let handles = match unsafe {
            self.device
                .inner
                .create_graphics_pipelines(self.handle, create_infos, None)
        } {
            Ok(inner) => inner,
            Err((pipelines, err)) => {
                for pipeline in pipelines {
                    if pipeline != vk::Pipeline::null() {
                        unsafe { self.device.inner.destroy_pipeline(pipeline, None) };
                    }
                }
                return Err(err);
            }
        };
        Ok(handles
            .into_iter()
            .map(|handle| Pipeline {
                handle,
                device: self.device.clone(),
            })
            .collect())
    }
}

impl RenderPass {
    pub fn create_framebuffer(
        &self,
        flags: FramebufferCreateFlags,
        attachments: &[&ImageView],
        width: u32,
        height: u32,
        layers: u32,
    ) -> VkResult<Framebuffer> {
        let attachments: Vec<_> = attachments
            .iter()
            .map(|attachment| attachment.handle)
            .collect();
        let create_info = FramebufferCreateInfo::builder()
            .flags(flags)
            .render_pass(self.handle)
            .attachments(&attachments)
            .width(width)
            .height(height)
            .layers(layers);
        let handle = unsafe { self.device.inner.create_framebuffer(&create_info, None) }?;
        Ok(Framebuffer {
            handle,
            device: self.device.clone(),
        })
    }
}
