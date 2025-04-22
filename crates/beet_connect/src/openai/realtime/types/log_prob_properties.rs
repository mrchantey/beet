use serde::{Deserialize, Serialize};

/// LogProbProperties : A log probability object. 
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogProbProperties {
    /// The token that was used to generate the log probability. 
    #[serde(rename = "token")]
    pub token: String,
    /// The log probability of the token. 
    #[serde(rename = "logprob")]
    pub logprob: f64,
    /// The bytes that were used to generate the log probability. 
    #[serde(rename = "bytes")]
    pub bytes: Vec<i32>,
}

impl LogProbProperties {
    /// A log probability object. 
    pub fn new(token: String, logprob: f64, bytes: Vec<i32>) -> LogProbProperties {
        LogProbProperties {
            token,
            logprob,
            bytes,
        }
    }
}

