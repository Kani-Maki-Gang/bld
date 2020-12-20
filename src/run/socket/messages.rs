use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ExecutePipelineSocketMessage(pub String);
