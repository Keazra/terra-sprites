use serde::{Deserialize, Serialize};

/// The kind of ground a tile has (design §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Grass,
    Dirt,
    Sand,
    ShallowWater,
    DeepWater,
    Rock,
}

impl Terrain {
    /// Every terrain, in declaration order.
    pub const ALL: [Terrain; 6] = [
        Terrain::Grass,
        Terrain::Dirt,
        Terrain::Sand,
        Terrain::ShallowWater,
        Terrain::DeepWater,
        Terrain::Rock,
    ];
}

/// A terrain's properties, from the data pack's `terrain.ron`.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainProps {
    pub(crate) step_cost: Option<u16>,
    pub(crate) fertility: f32,
    pub(crate) drinkable: bool,
}

impl TerrainProps {
    /// What an orthogonal step onto this terrain costs, or `None` if it isn't walkable.
    pub fn step_cost(&self) -> Option<u16> {
        self.step_cost
    }

    /// How well plants grow here, from 0 to 1. Always 0 on unwalkable terrain.
    pub fn fertility(&self) -> f32 {
        self.fertility
    }

    /// Whether sprites can drink here.
    pub fn is_drinkable(&self) -> bool {
        self.drinkable
    }
}
