pub type HandleFlow = Result<(), ()>;

pub type HandleFlowResult<E> = Result<(), Result<(), E>>;

pub trait HandleFlowExt {
    fn handled() -> Self;

    fn unhandled() -> Self;

    fn into_flow_result<E>(self) -> HandleFlowResult<E>;
}

impl HandleFlowExt for HandleFlow {
    fn handled() -> Self {
        Err(())
    }

    fn unhandled() -> Self {
        Ok(())
    }

    fn into_flow_result<E>(self) -> HandleFlowResult<E> {
        self.map_err(Ok)
    }
}

pub trait HandleFlowResultExt {
    type Error;

    fn handled() -> Self;

    fn unhandled() -> Self;

    fn error(error: Self::Error) -> Self;

    fn into_result(self) -> Result<(), Self::Error>;
}

impl<E> HandleFlowResultExt for HandleFlowResult<E> {
    type Error = E;

    fn handled() -> Self {
        Err(Ok(()))
    }

    fn unhandled() -> Self {
        Ok(())
    }

    fn error(error: Self::Error) -> Self {
        Err(Err(error))
    }

    fn into_result(self) -> Result<(), Self::Error> {
        match self {
            Ok(()) => Ok(()),
            Err(result) => result,
        }
    }
}

pub trait ResultExt {
    type Output;

    fn into_handled(self) -> Self::Output;
    fn into_unhandled(self) -> Self::Output;
}

impl<E> ResultExt for Result<(), E> {
    type Output = HandleFlowResult<E>;

    fn into_handled(self) -> Self::Output {
        Result::Err(self)
    }

    fn into_unhandled(self) -> Self::Output {
        self.map_err(Result::Err)
    }
}
