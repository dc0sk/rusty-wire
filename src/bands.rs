/// Define ham radio and shortwave bands with their characteristics
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandType {
    HF, // High Frequency (3-30 MHz)
    MF, // Medium Frequency (300 kHz - 3 MHz)
}

impl fmt::Display for BandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            BandType::HF => "HF",
            BandType::MF => "MF",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ITURegion {
    Region1, // Europe, Africa, Middle East
    Region2, // Americas
    Region3, // Asia-Pacific
}

impl fmt::Display for ITURegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ITURegion::Region1 => "1 (Europe, Africa, Middle East)",
            ITURegion::Region2 => "2 (Americas)",
            ITURegion::Region3 => "3 (Asia-Pacific)",
        };
        write!(f, "Region {label}")
    }
}

impl ITURegion {
    pub fn short_name(&self) -> &'static str {
        match self {
            ITURegion::Region1 => "1",
            ITURegion::Region2 => "2",
            ITURegion::Region3 => "3",
        }
    }

    pub fn long_name(&self) -> &'static str {
        match self {
            ITURegion::Region1 => "Europe, Africa, Middle East",
            ITURegion::Region2 => "Americas",
            ITURegion::Region3 => "Asia-Pacific",
        }
    }
}

impl FromStr for ITURegion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "1" => Ok(ITURegion::Region1),
            "2" => Ok(ITURegion::Region2),
            "3" => Ok(ITURegion::Region3),
            _ => Err(format!("Invalid ITU region '{s}'. Must be 1, 2, or 3.")),
        }
    }
}

pub const ALL_REGIONS: &[ITURegion] = &[ITURegion::Region1, ITURegion::Region2, ITURegion::Region3];

#[derive(Debug, Clone)]
pub struct Band {
    pub name: &'static str,
    pub band_type: BandType,
    pub freq_low_mhz: f64,
    pub freq_high_mhz: f64,
    pub freq_center_mhz: f64,
    pub typical_skip_km: (f64, f64), // (min, max) skip distance in km
    pub regions: &'static [ITURegion], // Available in these ITU regions
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}] ({}-{} MHz)",
            self.name, self.band_type, self.freq_low_mhz, self.freq_high_mhz
        )
    }
}

fn region_adjusted_range(index: usize, region: ITURegion) -> Option<(f64, f64)> {
    match index {
        // 80m amateur allocation differs by ITU region.
        1 => Some(match region {
            ITURegion::Region1 => (3.5, 3.8),
            ITURegion::Region2 => (3.5, 4.0),
            ITURegion::Region3 => (3.5, 3.9),
        }),
        // 60m WRC-15 band segment used as shared baseline.
        2 => Some((5.3515, 5.3665)),
        // 40m differs by ITU region.
        3 => Some(match region {
            ITURegion::Region1 => (7.0, 7.2),
            ITURegion::Region2 => (7.0, 7.3),
            ITURegion::Region3 => (7.0, 7.2),
        }),
        _ => None,
    }
}

fn band_for_region(base: &Band, index: usize, region: ITURegion) -> Band {
    if let Some((low, high)) = region_adjusted_range(index, region) {
        let mut adjusted = base.clone();
        adjusted.freq_low_mhz = low;
        adjusted.freq_high_mhz = high;
        adjusted.freq_center_mhz = (low + high) / 2.0;
        adjusted
    } else {
        base.clone()
    }
}

/// All available ham radio and shortwave bands
pub const BANDS: &[Band] = &[
    // HF Bands (amateur radio)
    Band {
        name: "160m",
        band_type: BandType::HF,
        freq_low_mhz: 1.8,
        freq_high_mhz: 2.0,
        freq_center_mhz: 1.9,
        typical_skip_km: (100.0, 2000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "80m",
        band_type: BandType::HF,
        freq_low_mhz: 3.5,
        freq_high_mhz: 4.0,
        freq_center_mhz: 3.75,
        typical_skip_km: (50.0, 1500.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "60m",
        band_type: BandType::HF,
        freq_low_mhz: 5.3515,
        freq_high_mhz: 5.3665,
        freq_center_mhz: 5.359,
        typical_skip_km: (50.0, 1200.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "40m",
        band_type: BandType::HF,
        freq_low_mhz: 7.0,
        freq_high_mhz: 7.3,
        freq_center_mhz: 7.15,
        typical_skip_km: (50.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "30m",
        band_type: BandType::HF,
        freq_low_mhz: 10.1,
        freq_high_mhz: 10.15,
        freq_center_mhz: 10.125,
        typical_skip_km: (100.0, 800.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "20m",
        band_type: BandType::HF,
        freq_low_mhz: 14.0,
        freq_high_mhz: 14.35,
        freq_center_mhz: 14.175,
        typical_skip_km: (150.0, 800.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "17m",
        band_type: BandType::HF,
        freq_low_mhz: 18.068,
        freq_high_mhz: 18.168,
        freq_center_mhz: 18.118,
        typical_skip_km: (150.0, 800.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "15m",
        band_type: BandType::HF,
        freq_low_mhz: 21.0,
        freq_high_mhz: 21.45,
        freq_center_mhz: 21.225,
        typical_skip_km: (200.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "12m",
        band_type: BandType::HF,
        freq_low_mhz: 24.89,
        freq_high_mhz: 24.99,
        freq_center_mhz: 24.94,
        typical_skip_km: (200.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "10m",
        band_type: BandType::HF,
        freq_low_mhz: 28.0,
        freq_high_mhz: 29.7,
        freq_center_mhz: 28.85,
        typical_skip_km: (250.0, 1200.0),
        regions: ALL_REGIONS,
    },
    // Shortwave broadcast bands
    Band {
        name: "120m SW (2.3-2.495 MHz)",
        band_type: BandType::MF,
        freq_low_mhz: 2.3,
        freq_high_mhz: 2.495,
        freq_center_mhz: 2.398,
        typical_skip_km: (100.0, 1500.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "90m SW (3.2-3.4 MHz)",
        band_type: BandType::MF,
        freq_low_mhz: 3.2,
        freq_high_mhz: 3.4,
        freq_center_mhz: 3.3,
        typical_skip_km: (100.0, 1200.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "75m SW (3.9-4.0 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 3.9,
        freq_high_mhz: 4.0,
        freq_center_mhz: 3.95,
        typical_skip_km: (50.0, 1500.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "49m SW (5.9-6.2 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 5.9,
        freq_high_mhz: 6.2,
        freq_center_mhz: 6.05,
        typical_skip_km: (50.0, 1200.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "41m SW (7.2-7.45 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 7.2,
        freq_high_mhz: 7.45,
        freq_center_mhz: 7.325,
        typical_skip_km: (50.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "31m SW (9.4-9.9 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 9.4,
        freq_high_mhz: 9.9,
        freq_center_mhz: 9.65,
        typical_skip_km: (100.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "25m SW (11.6-12.1 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 11.6,
        freq_high_mhz: 12.1,
        freq_center_mhz: 11.85,
        typical_skip_km: (200.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "22m SW (13.57-13.87 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 13.57,
        freq_high_mhz: 13.87,
        freq_center_mhz: 13.72,
        typical_skip_km: (200.0, 1000.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "19m SW (15.1-15.8 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 15.1,
        freq_high_mhz: 15.8,
        freq_center_mhz: 15.45,
        typical_skip_km: (250.0, 1200.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "16m SW (17.48-17.9 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 17.48,
        freq_high_mhz: 17.9,
        freq_center_mhz: 17.69,
        typical_skip_km: (250.0, 1200.0),
        regions: ALL_REGIONS,
    },
    Band {
        name: "13m SW (21.45-21.85 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 21.45,
        freq_high_mhz: 21.85,
        freq_center_mhz: 21.65,
        typical_skip_km: (250.0, 1200.0),
        regions: ALL_REGIONS,
    },
];

pub fn get_bands_for_region(region: ITURegion) -> Vec<(usize, Band)> {
    BANDS
        .iter()
        .enumerate()
        .filter(|(_, band)| band.regions.contains(&region))
        .map(|(idx, band)| (idx, band_for_region(band, idx, region)))
        .collect()
}

pub fn get_band_by_index_for_region(index: usize, region: ITURegion) -> Option<Band> {
    BANDS.get(index).and_then(|band| {
        if band.regions.contains(&region) {
            Some(band_for_region(band, index, region))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itu_region_display() {
        assert_eq!(ITURegion::Region1.short_name(), "1");
        assert_eq!(ITURegion::Region2.short_name(), "2");
        assert_eq!(ITURegion::Region3.short_name(), "3");

        assert_eq!(
            ITURegion::Region1.long_name(),
            "Europe, Africa, Middle East"
        );
        assert_eq!(ITURegion::Region2.long_name(), "Americas");
        assert_eq!(ITURegion::Region3.long_name(), "Asia-Pacific");
    }

    #[test]
    fn band_type_display() {
        assert_eq!(BandType::HF.to_string(), "HF");
        assert_eq!(BandType::MF.to_string(), "MF");
    }

    #[test]
    fn band_display() {
        let band = Band {
            name: "20m",
            band_type: BandType::HF,
            freq_low_mhz: 14.0,
            freq_high_mhz: 14.35,
            freq_center_mhz: 14.175,
            typical_skip_km: (150.0, 800.0),
            regions: &[ITURegion::Region1],
        };
        let display = band.to_string();
        assert!(display.contains("20m"));
        assert!(display.contains("HF"));
        assert!(display.contains("14"));
    }

    #[test]
    fn get_bands_for_region1() {
        let bands = get_bands_for_region(ITURegion::Region1);
        assert!(!bands.is_empty());

        let band_names: Vec<&str> = bands.iter().map(|(_, b)| b.name).collect();
        assert!(band_names.contains(&"20m"));
        assert!(band_names.contains(&"40m"));
    }

    #[test]
    fn get_bands_for_region2() {
        let bands = get_bands_for_region(ITURegion::Region2);
        assert!(!bands.is_empty());

        let band_names: Vec<&str> = bands.iter().map(|(_, b)| b.name).collect();
        assert!(band_names.contains(&"20m"));
    }

    #[test]
    fn get_bands_for_region3() {
        let bands = get_bands_for_region(ITURegion::Region3);
        assert!(!bands.is_empty());

        let band_names: Vec<&str> = bands.iter().map(|(_, b)| b.name).collect();
        assert!(band_names.contains(&"40m"));
    }

    #[test]
    fn get_band_by_index_valid() {
        let band = get_band_by_index_for_region(0, ITURegion::Region1);
        assert!(band.is_some());
        assert_eq!(band.unwrap().name, "160m");
    }

    #[test]
    fn get_band_by_index_invalid() {
        let band = get_band_by_index_for_region(9999, ITURegion::Region1);
        assert!(band.is_none());
    }

    #[test]
    fn band_80m_region_adjustment() {
        // 80m (index 1) has region-specific frequencies
        let r1 = get_band_by_index_for_region(1, ITURegion::Region1).unwrap();
        let r2 = get_band_by_index_for_region(1, ITURegion::Region2).unwrap();
        let r3 = get_band_by_index_for_region(1, ITURegion::Region3).unwrap();

        // Region 1: 3.5-3.8
        assert_eq!(r1.freq_low_mhz, 3.5);
        assert_eq!(r1.freq_high_mhz, 3.8);

        // Region 2: 3.5-4.0
        assert_eq!(r2.freq_low_mhz, 3.5);
        assert_eq!(r2.freq_high_mhz, 4.0);

        // Region 3: 3.5-3.9
        assert_eq!(r3.freq_low_mhz, 3.5);
        assert_eq!(r3.freq_high_mhz, 3.9);
    }

    #[test]
    fn band_40m_region_adjustment() {
        // 40m (index 3) has region-specific frequencies
        let r1 = get_band_by_index_for_region(3, ITURegion::Region1).unwrap();
        let r2 = get_band_by_index_for_region(3, ITURegion::Region2).unwrap();
        let r3 = get_band_by_index_for_region(3, ITURegion::Region3).unwrap();

        // Region 1: 7.0-7.2
        assert_eq!(r1.freq_low_mhz, 7.0);
        assert_eq!(r1.freq_high_mhz, 7.2);

        // Region 2: 7.0-7.3
        assert_eq!(r2.freq_low_mhz, 7.0);
        assert_eq!(r2.freq_high_mhz, 7.3);

        // Region 3: 7.0-7.2
        assert_eq!(r3.freq_low_mhz, 7.0);
        assert_eq!(r3.freq_high_mhz, 7.2);
    }

    #[test]
    fn band_60m_harmonized() {
        // 60m (index 2) should be consistent across regions
        let r1 = get_band_by_index_for_region(2, ITURegion::Region1).unwrap();
        let r2 = get_band_by_index_for_region(2, ITURegion::Region2).unwrap();
        let r3 = get_band_by_index_for_region(2, ITURegion::Region3).unwrap();

        assert_eq!(r1.freq_low_mhz, r2.freq_low_mhz);
        assert_eq!(r1.freq_high_mhz, r2.freq_high_mhz);
        assert_eq!(r1.freq_low_mhz, r3.freq_low_mhz);
        assert_eq!(r1.freq_high_mhz, r3.freq_high_mhz);
    }

    #[test]
    fn all_regions_constant() {
        assert_eq!(ALL_REGIONS.len(), 3);
        assert_eq!(ALL_REGIONS[0], ITURegion::Region1);
        assert_eq!(ALL_REGIONS[1], ITURegion::Region2);
        assert_eq!(ALL_REGIONS[2], ITURegion::Region3);
    }
}
