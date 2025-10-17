use serde::{Deserialize, Serialize};
use serde_json::json;

pub const ENTITY_TYPE_VARIANTS: [&str; 37] = [
    "Researcher",
    "Clinician",
    "Patient / Participant",
    "Institution / Organization",
    "Funding Agency",
    "Gene",
    "Protein",
    "RNA",
    "Cell",
    "Tissue",
    "Organ",
    "Organism / Species",
    "Disease / Disorder",
    "Syndrome",
    "Symptom / Phenotype",
    "Pathway",
    "Drug / Compound / Chemical Substance",
    "Biomarker",
    "Reagent",
    "Material",
    "Method / Technique / Assay / Protocol",
    "Equipment / Instrument",
    "Sample / Specimen",
    "Control / Variable",
    "Measurement / Metric",
    "Dataset",
    "Model (computational, statistical, or biological)",
    "Hypothesis / Objective",
    "Result / Observation / Finding",
    "Theory / Concept",
    "Parameter",
    "Clinical Trial",
    "Project / Study",
    "Ethical Approval / Consent",
    "Time / Duration / Temporal Stage",
    "Location",
    "Publication / Reference",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum EntityType {
    #[serde(rename = "Researcher")]
    Researcher,
    #[serde(rename = "Clinician")]
    Clinician,
    #[serde(rename = "Patient / Participant")]
    PatientParticipant,
    #[serde(rename = "Institution / Organization")]
    InstitutionOrganization,
    #[serde(rename = "Funding Agency")]
    FundingAgency,
    #[serde(rename = "Gene")]
    Gene,
    #[serde(rename = "Protein")]
    Protein,
    #[serde(rename = "RNA")]
    Rna,
    #[serde(rename = "Cell")]
    Cell,
    #[serde(rename = "Tissue")]
    Tissue,
    #[serde(rename = "Organ")]
    Organ,
    #[serde(rename = "Organism / Species")]
    OrganismSpecies,
    #[serde(rename = "Disease / Disorder")]
    DiseaseDisorder,
    #[serde(rename = "Syndrome")]
    Syndrome,
    #[serde(rename = "Symptom / Phenotype")]
    SymptomPhenotype,
    #[serde(rename = "Pathway")]
    PathwayMetabolicOrSignaling,
    #[serde(rename = "Drug / Compound / Chemical Substance")]
    DrugCompoundChemicalSubstance,
    #[serde(rename = "Biomarker")]
    Biomarker,
    #[serde(rename = "Reagent")]
    Reagent,
    #[serde(rename = "Material")]
    MaterialScaffoldOrNanoparticle,
    #[serde(rename = "Method / Technique / Assay / Protocol")]
    MethodTechniqueAssayProtocol,
    #[serde(rename = "Equipment / Instrument")]
    EquipmentInstrument,
    #[serde(rename = "Sample / Specimen")]
    SampleSpecimen,
    #[serde(rename = "Control / Variable")]
    ControlVariable,
    #[serde(rename = "Measurement / Metric")]
    MeasurementMetric,
    #[serde(rename = "Dataset")]
    Dataset,
    #[serde(rename = "Model (computational, statistical, or biological)")]
    ModelComputationalStatisticalOrBiological,
    #[serde(rename = "Hypothesis / Objective")]
    HypothesisObjective,
    #[serde(rename = "Result / Observation / Finding")]
    ResultObservationFinding,
    #[serde(rename = "Theory / Concept")]
    TheoryConcept,
    #[serde(rename = "Parameter")]
    Parameter,
    #[serde(rename = "Clinical Trial")]
    ClinicalTrial,
    #[serde(rename = "Project / Study")]
    ProjectStudy,
    #[serde(rename = "Ethical Approval / Consent")]
    EthicalApprovalConsent,
    #[serde(rename = "Time / Duration / Temporal Stage")]
    TimeDurationTemporalStage,
    #[serde(rename = "Location")]
    LocationResearchSiteHospitalRegion,
    #[serde(rename = "Publication / Reference")]
    PublicationReference,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Researcher => "Researcher",
            Self::Clinician => "Clinician",
            Self::PatientParticipant => "Patient / Participant",
            Self::InstitutionOrganization => "Institution / Organization",
            Self::FundingAgency => "Funding Agency",
            Self::Gene => "Gene",
            Self::Protein => "Protein",
            Self::Rna => "RNA",
            Self::Cell => "Cell",
            Self::Tissue => "Tissue",
            Self::Organ => "Organ",
            Self::OrganismSpecies => "Organism / Species",
            Self::DiseaseDisorder => "Disease / Disorder",
            Self::Syndrome => "Syndrome",
            Self::SymptomPhenotype => "Symptom / Phenotype",
            Self::PathwayMetabolicOrSignaling => "Pathway",
            Self::DrugCompoundChemicalSubstance => "Drug / Compound / Chemical Substance",
            Self::Biomarker => "Biomarker",
            Self::Reagent => "Reagent",
            Self::MaterialScaffoldOrNanoparticle => "Material",
            Self::MethodTechniqueAssayProtocol => "Method / Technique / Assay / Protocol",
            Self::EquipmentInstrument => "Equipment / Instrument",
            Self::SampleSpecimen => "Sample / Specimen",
            Self::ControlVariable => "Control / Variable",
            Self::MeasurementMetric => "Measurement / Metric",
            Self::Dataset => "Dataset",
            Self::ModelComputationalStatisticalOrBiological => {
                "Model (computational, statistical, or biological)"
            }
            Self::HypothesisObjective => "Hypothesis / Objective",
            Self::ResultObservationFinding => "Result / Observation / Finding",
            Self::TheoryConcept => "Theory / Concept",
            Self::Parameter => "Parameter",
            Self::ClinicalTrial => "Clinical Trial",
            Self::ProjectStudy => "Project / Study",
            Self::EthicalApprovalConsent => "Ethical Approval / Consent",
            Self::TimeDurationTemporalStage => "Time / Duration / Temporal Stage",
            Self::LocationResearchSiteHospitalRegion => "Location",
            Self::PublicationReference => "Publication / Reference",
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                            "enum": ENTITY_TYPE_VARIANTS.iter().copied().collect::<Vec<_>>(),
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
