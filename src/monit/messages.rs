use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct MonitorPipelineSocketMessage(pub String);
