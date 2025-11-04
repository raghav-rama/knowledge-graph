use serde::{Deserialize, Serialize};
use serde_json::json;

pub const BASE_ENTITY_TYPES: [&str; 14] = [
    "Gene",
    "Protein",
    "Compound",
    "BiologicalProcess",
    "MolecularFunction",
    "CellularComponent",
    "Pathway",
    "Disease",
    "Symptom",
    "Intervention",
    "Mechanism",
    "CellType",
    "Tissue",
    "Organism",
];

pub const LONGEVITY_EXTENSION: [&str; 3] = ["AgingHallmark", "Biomarker", "LifespanModel"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum EntityType {
    #[serde(rename = "Gene")]
    Gene,
    #[serde(rename = "Disease")]
    Disease,
    #[serde(rename = "Pathway")]
    Pathway,
    #[serde(rename = "PharmacologicClass")]
    PharmacologicClass,
    #[serde(rename = "CellularComponent")]
    CellularComponent,
    #[serde(rename = "Compound")]
    Compound,
    #[serde(rename = "Anatomy")]
    Anatomy,
    #[serde(rename = "Symptom")]
    Symptom,
    #[serde(rename = "BiologicalProcess")]
    BiologicalProcess,
    #[serde(rename = "MolecularFunction")]
    MolecularFunction,
    #[serde(rename = "SideEffect")]
    SideEffect,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gene => "Gene",
            Self::Disease => "Disease",
            Self::Pathway => "Pathway",
            Self::PharmacologicClass => "PharmacologicClass",
            Self::CellularComponent => "CellularComponent",
            Self::Compound => "Compound",
            Self::Anatomy => "Anatomy",
            Self::Symptom => "Symptom",
            Self::BiologicalProcess => "BiologicalProcess",
            Self::MolecularFunction => "MolecularFunction",
            Self::SideEffect => "SideEffect",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub entity_name: String,
    pub entity_type: EntityType,
    pub entity_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub source_entity: String,
    pub target_entity: String,
    pub relationship_keywords: Vec<String>,
    pub relationship_description: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EntitiesRelationships {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
}

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

pub fn entities_relationships_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "entity_name": {
                            "type": "string",
                            "description": "The name of the entity. If the entity name is case-insensitive, capitalize the first letter of each significant word (title case). Ensure **consistent naming** across the entire extraction process."
                        },
                        "entity_type": {
                            "type": "string",
                            "enum": BASE_ENTITY_TYPES.iter().copied().collect::<Vec<_>>(),
                            "description": "Categorize the entity using one of the following controlled vocabulary. If none of the provided entity types apply, do not add new entity type and classify it as `Other`"
                        },
                        "entity_description": {
                            "type": "string",
                            "description": "Provide a concise yet comprehensive description of the entity's attributes and activities, based *solely* on the information present in the input text."
                        }
                    },
                    "required": ["entity_name", "entity_type", "entity_description"]
                }
            },
            "relationships": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "source_entity": {
                            "type": "string",
                            "description": "The name of the source entity. Ensure **consistent naming** with entity extraction. Capitalize the first letter of each significant word (title case) if the name is case-insensitive."
                        },
                        "target_entity": {
                            "type": "string",
                            "description": "The name of the target entity. Ensure **consistent naming** with entity extraction. Capitalize the first letter of each significant word (title case) if the name is case-insensitive."
                        },
                        "relationship_keywords": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "description": "Oßne or more high-level keywords summarizing the overarching nature, concepts, or themes of the relationship. Multiple keywords within this field must be separated by a comma `,`. **DO NOT use `{tuple_delimiter}` for separating multiple keywords within this field.**"
                            }
                        },
                        "relationship_description":  {
                            "type": "string",
                            "description": "A concise explanation of the nature of the relationship between the source and target entities, providing a clear rationale for their connection."
                        }
                    },
                    "required": ["source_entity", "target_entity", "relationship_keywords", "relationship_description"]
                }
            }
        },
        "required": ["entities", "relationships"]
    })
}
