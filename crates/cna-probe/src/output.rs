//! The probe file format (`probes/<id>.json`), read by the `cna` board.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Line,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    pub label: String,
    /// Category labels on x; empty on y.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    /// The ruling option this series shows, or none for a baseline.
    pub option: Option<u8>,
    pub label: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Probe {
    /// A ruling id or a decision-board item id.
    pub id: String,
    pub kind: Kind,
    pub question: String,
    pub rules_commit: String,
    pub engine_commit: String,
    pub x: Axis,
    pub y: Axis,
    pub series: Vec<Series>,
    /// Written from the numbers by the probe's code.
    pub finding: String,
}
