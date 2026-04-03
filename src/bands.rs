/// Define ham radio and shortwave bands with their characteristics
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandType {
    HF,     // High Frequency (3-30 MHz)
    MF,     // Medium Frequency (300 kHz - 3 MHz)
}

impl fmt::Display for BandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            BandType::HF => "HF",
            BandType::MF => "MF",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug, Clone)]
pub struct Band {
    pub name: &'static str,
    pub band_type: BandType,
    pub freq_low_mhz: f64,
    pub freq_high_mhz: f64,
    pub freq_center_mhz: f64,
    pub typical_skip_km: (f64, f64), // (min, max) skip distance in km
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

/// All available ham radio and shortwave bands
pub const BANDS: &[Band] = &[
    // HF Bands (amateur radio)
    Band {
        name: "160m (1.8-2.0 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 1.8,
        freq_high_mhz: 2.0,
        freq_center_mhz: 1.9,
        typical_skip_km: (100.0, 2000.0),
    },
    Band {
        name: "80m (3.5-4.0 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 3.5,
        freq_high_mhz: 4.0,
        freq_center_mhz: 3.75,
        typical_skip_km: (50.0, 1500.0),
    },
    Band {
        name: "60m (5.25-5.45 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 5.25,
        freq_high_mhz: 5.45,
        freq_center_mhz: 5.35,
        typical_skip_km: (50.0, 1200.0),
    },
    Band {
        name: "40m (7.0-7.3 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 7.0,
        freq_high_mhz: 7.3,
        freq_center_mhz: 7.15,
        typical_skip_km: (50.0, 1000.0),
    },
    Band {
        name: "30m (10.1-10.15 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 10.1,
        freq_high_mhz: 10.15,
        freq_center_mhz: 10.125,
        typical_skip_km: (100.0, 800.0),
    },
    Band {
        name: "20m (14.0-14.35 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 14.0,
        freq_high_mhz: 14.35,
        freq_center_mhz: 14.175,
        typical_skip_km: (150.0, 800.0),
    },
    Band {
        name: "17m (18.068-18.168 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 18.068,
        freq_high_mhz: 18.168,
        freq_center_mhz: 18.118,
        typical_skip_km: (150.0, 800.0),
    },
    Band {
        name: "15m (21.0-21.45 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 21.0,
        freq_high_mhz: 21.45,
        freq_center_mhz: 21.225,
        typical_skip_km: (200.0, 1000.0),
    },
    Band {
        name: "12m (24.89-24.99 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 24.89,
        freq_high_mhz: 24.99,
        freq_center_mhz: 24.94,
        typical_skip_km: (200.0, 1000.0),
    },
    Band {
        name: "10m (28.0-29.7 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 28.0,
        freq_high_mhz: 29.7,
        freq_center_mhz: 28.85,
        typical_skip_km: (250.0, 1200.0),
    },
    // Shortwave broadcast bands
    Band {
        name: "120m SW (2.3-2.495 MHz)",
        band_type: BandType::MF,
        freq_low_mhz: 2.3,
        freq_high_mhz: 2.495,
        freq_center_mhz: 2.398,
        typical_skip_km: (100.0, 1500.0),
    },
    Band {
        name: "90m SW (3.2-3.4 MHz)",
        band_type: BandType::MF,
        freq_low_mhz: 3.2,
        freq_high_mhz: 3.4,
        freq_center_mhz: 3.3,
        typical_skip_km: (100.0, 1200.0),
    },
    Band {
        name: "75m SW (3.9-4.0 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 3.9,
        freq_high_mhz: 4.0,
        freq_center_mhz: 3.95,
        typical_skip_km: (50.0, 1500.0),
    },
    Band {
        name: "49m SW (5.9-6.2 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 5.9,
        freq_high_mhz: 6.2,
        freq_center_mhz: 6.05,
        typical_skip_km: (50.0, 1200.0),
    },
    Band {
        name: "41m SW (7.2-7.45 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 7.2,
        freq_high_mhz: 7.45,
        freq_center_mhz: 7.325,
        typical_skip_km: (50.0, 1000.0),
    },
    Band {
        name: "31m SW (9.4-9.9 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 9.4,
        freq_high_mhz: 9.9,
        freq_center_mhz: 9.65,
        typical_skip_km: (100.0, 1000.0),
    },
    Band {
        name: "25m SW (11.6-12.1 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 11.6,
        freq_high_mhz: 12.1,
        freq_center_mhz: 11.85,
        typical_skip_km: (200.0, 1000.0),
    },
    Band {
        name: "22m SW (13.57-13.87 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 13.57,
        freq_high_mhz: 13.87,
        freq_center_mhz: 13.72,
        typical_skip_km: (200.0, 1000.0),
    },
    Band {
        name: "19m SW (15.1-15.8 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 15.1,
        freq_high_mhz: 15.8,
        freq_center_mhz: 15.45,
        typical_skip_km: (250.0, 1200.0),
    },
    Band {
        name: "16m SW (17.48-17.9 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 17.48,
        freq_high_mhz: 17.9,
        freq_center_mhz: 17.69,
        typical_skip_km: (250.0, 1200.0),
    },
    Band {
        name: "13m SW (21.45-21.85 MHz)",
        band_type: BandType::HF,
        freq_low_mhz: 21.45,
        freq_high_mhz: 21.85,
        freq_center_mhz: 21.65,
        typical_skip_km: (250.0, 1200.0),
    },
];

pub fn get_band_by_index(index: usize) -> Option<&'static Band> {
    BANDS.get(index)
}

pub fn band_count() -> usize {
    BANDS.len()
}
