use super::*;

pub struct MessageValidator {
    kind: Option<MessageKind>,
    task: Option<Task>,
    sender: Target,
}

impl MessageValidator {
    pub fn new(sender: Target) -> Self {
        Self::builder(sender).build()
    }

    pub fn builder(sender: Target) -> MessageValidatorBuilder {
        MessageValidatorBuilder::new(sender)
    }
}

#[cfg(feature = "backend")]
impl MessageValidator {
    pub fn validate(&self, msg: &Message) -> anyhow::Result<()> {
        if msg.sender == self.sender
            && msg.target == Message::CURRENT_TARGET
            && self.kind.is_none_or(|kind| kind == msg.kind)
            && self.task.is_none_or(|task| task == msg.task)
        {
            return Ok(());
        }

        Err(anyhow::anyhow!("Could not validate message!"))
    }
}

#[cfg(feature = "extension")]
impl MessageValidator {
    #[track_caller]
    pub fn validate(&self, msg: &Message) -> Result<(), JsValue> {
        if msg.sender == self.sender
            && msg.target == Message::CURRENT_TARGET
            && self.kind.is_none_or(|kind| kind == msg.kind)
            && self.task.is_none_or(|task| task == msg.task)
        {
            return Ok(());
        }

        Err(crate::err_code!(track_caller))
    }
}

pub struct MessageValidatorBuilder {
    kind: Option<MessageKind>,
    task: Option<Task>,
    sender: Target,
}

impl MessageValidatorBuilder {
    fn new(sender: Target) -> Self {
        Self {
            kind: Some(MessageKind::Request),
            task: None,
            sender,
        }
    }

    pub fn maybe_kind(mut self, kind: Option<MessageKind>) -> Self {
        self.kind = kind;
        self
    }

    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn task(mut self, task: Task) -> Self {
        self.task = Some(task);
        self
    }

    pub fn build(self) -> MessageValidator {
        MessageValidator {
            kind: self.kind,
            task: self.task,
            sender: self.sender,
        }
    }
}
