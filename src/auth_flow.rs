use tokio::sync::oneshot;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthFlowState {
    AwaitingPassword { prompt: String },
    Verifying,
    Success,
    Failed { message: String },
}

pub struct AuthFlow {
    pub state: AuthFlowState,
    pub message: String,
    pub password: String,
    responder: Option<oneshot::Sender<Option<String>>>,
}

impl AuthFlow {
    pub fn new(
        prompt: String,
        message: String,
        responder: oneshot::Sender<Option<String>>,
    ) -> Self {
        Self {
            state: AuthFlowState::AwaitingPassword { prompt },
            message,
            password: String::new(),
            responder: Some(responder),
        }
    }

    pub fn set_password(&mut self, value: String) {
        self.clear_password();
        self.password = value;
    }

    pub fn submit(&mut self) -> Option<String> {
        let password = self.password.clone();
        let responder = self.responder.take();

        self.clear_password();

        if let Some(sender) = responder {
            let _ = sender.send(Some(password.clone()));
            self.state = AuthFlowState::Verifying;
            Some(password)
        } else {
            None
        }
    }

    pub fn cancel(&mut self) {
        self.clear_password();

        if let Some(sender) = self.responder.take() {
            let _ = sender.send(None);
        }

        self.state = AuthFlowState::Failed {
            message: "Authentication cancelled".to_string(),
        };
    }

    fn clear_password(&mut self) {
        self.password.zeroize();
        self.password.clear();
    }
}

impl Drop for AuthFlow {
    fn drop(&mut self) {
        self.clear_password();
    }
}
