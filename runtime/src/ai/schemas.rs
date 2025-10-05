use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub name: String,
    pub date: String,
    pub participants: Vec<String>,
}

pub fn calendar_event_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": { "type": "string" },
            "date": { "type": "string" },
            "participants": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["name", "date", "participants"]
    })
}
