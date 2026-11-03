#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Terrain {
    DeepWater,
    ShallowWater,
    Sand,
    Grass,
}

impl Terrain {
    pub fn is_water(self) -> bool {
        matches!(self, Terrain::DeepWater | Terrain::ShallowWater)
    }

    pub fn is_land(self) -> bool {
        !self.is_water()
    }
}

/// Which quarter of a 2x2 palm tree a cell holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PalmPart {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Objects placed on top of terrain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    None,
    Palm(PalmPart),
    Rock,
    Bush,
    Driftwood,
    Shell,
    Flower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileProps {
    pub solid: bool,
    /// Walkable but slow, and the walker's legs are hidden.
    pub wading: bool,
}

impl TileProps {
    pub const SOLID: Self = Self { solid: true, wading: false };
}

pub fn tile_props(terrain: Terrain, feature: Feature) -> TileProps {
    let feature_solid = matches!(
        feature,
        Feature::Palm(_) | Feature::Rock | Feature::Bush | Feature::Driftwood
    );
    TileProps {
        solid: feature_solid || terrain == Terrain::DeepWater,
        wading: terrain == Terrain::ShallowWater,
    }
}
