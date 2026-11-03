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

/// Which cell of a 2x3 palm tree a map cell holds. Only the bottom row
/// (the trunk base) is solid; the canopy rows can be walked behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PalmPart {
    pub dx: u8,
    pub dy: u8,
    /// Mirrored tree (uses the PPU's h-flip, no extra tiles).
    pub flip: bool,
}

impl PalmPart {
    pub const W: i32 = 2;
    pub const H: i32 = 3;
}

/// One of the two cells of a palm stump (the old trunk-base row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StumpPart {
    pub dx: u8,
    pub flip: bool,
}

/// Objects placed on top of terrain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    None,
    Palm(PalmPart),
    Stump(StumpPart),
    Rock,
    Pebbles,
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
    let feature_solid = match feature {
        Feature::Palm(p) => p.dy as i32 == PalmPart::H - 1,
        Feature::Stump(_) | Feature::Rock | Feature::Bush | Feature::Driftwood => true,
        _ => false,
    };
    TileProps {
        solid: feature_solid || terrain == Terrain::DeepWater,
        wading: terrain == Terrain::ShallowWater,
    }
}
