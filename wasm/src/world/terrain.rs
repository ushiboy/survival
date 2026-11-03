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

/// Which cell of a 3x4 palm tree a map cell holds. Only the trunk base
/// (bottom centre) is solid; everything else can be walked behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PalmPart {
    pub dx: u8,
    pub dy: u8,
    /// Mirrored tree (uses the PPU's h-flip, no extra tiles).
    pub flip: bool,
}

impl PalmPart {
    pub const W: i32 = 3;
    pub const H: i32 = 4;

    pub fn is_trunk_base(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy as i32 == Self::H - 1
    }

    /// The canopy cell the coconuts hang in.
    pub fn is_crown(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy == 1
    }

    /// Top-left cell of the tree this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.dx as i32, y - self.dy as i32)
    }
}

/// A palm stump (left in the old trunk-base cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StumpPart {
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
        Feature::Palm(p) => p.is_trunk_base(),
        Feature::Stump(_) | Feature::Rock | Feature::Bush | Feature::Driftwood => true,
        _ => false,
    };
    TileProps {
        solid: feature_solid || terrain == Terrain::DeepWater,
        wading: terrain == Terrain::ShallowWater,
    }
}
