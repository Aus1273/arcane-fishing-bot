//! Versioned JSON boundary shared by desktop interfaces and offline tools.
//! Responses contain proposed actions only; this module never executes them.
use crate::{
    config::BotConfig,
    engine::{Action, Controller, Observation},
    replay::{self, Scenario, Trace},
};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub protocol_version: u32,
    #[serde(flatten)]
    pub command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Start {
        config: Box<BotConfig>,
    },
    Step {
        at_ms: u64,
        observation: Option<Observation>,
        focused: bool,
        #[serde(default)]
        stop: bool,
    },
    Status,
}

#[derive(Serialize)]
pub struct Response {
    pub protocol_version: u32,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Payload {
    Controller {
        at_ms: u64,
        controller: Box<Controller>,
        actions: Vec<Action>,
    },
    Replay {
        name: String,
        frames: Vec<Trace>,
    },
    Error {
        error: String,
    },
}

impl Response {
    pub fn error(error: impl ToString) -> Self {
        Self::new(Payload::Error {
            error: error.to_string(),
        })
    }

    fn new(payload: Payload) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            payload,
        }
    }
}

#[derive(Default)]
pub struct Session {
    controller: Option<Controller>,
    at_ms: u64,
}

impl Session {
    pub fn handle(&mut self, request: Request) -> Response {
        if request.protocol_version != PROTOCOL_VERSION {
            return Response::error(format!(
                "Unsupported protocol version {}; expected {PROTOCOL_VERSION}",
                request.protocol_version
            ));
        }
        let mut actions = vec![];
        match request.command {
            Command::Start { config } => {
                if let Err(error) = config.validate() {
                    return Response::error(error);
                }
                self.controller = Some(Controller::new(*config));
                self.at_ms = 0;
            }
            Command::Step {
                at_ms,
                observation,
                focused,
                stop,
            } => {
                let Some(controller) = &mut self.controller else {
                    return Response::error("Start a controller before sending observations");
                };
                if at_ms < self.at_ms {
                    return Response::error("Observation time must not move backwards");
                }
                actions = controller.step(at_ms, observation.as_ref(), focused, stop);
                self.at_ms = at_ms;
            }
            Command::Status => {}
        }
        match &self.controller {
            Some(controller) => Response::new(Payload::Controller {
                at_ms: self.at_ms,
                controller: Box::new(controller.clone()),
                actions,
            }),
            None => Response::error("No controller has been started"),
        }
    }

    pub fn handle_json(&mut self, json: &str) -> Response {
        match serde_json::from_str(json) {
            Ok(request) => self.handle(request),
            Err(error) => Response::error(format!("Invalid request: {error}")),
        }
    }
}

pub fn replay(scenario: Scenario) -> Response {
    let name = scenario.name.clone();
    match replay::run(scenario) {
        Ok(frames) => Response::new(Payload::Replay { name, frames }),
        Err(error) => Response::error(error),
    }
}
