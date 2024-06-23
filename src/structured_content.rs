use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

