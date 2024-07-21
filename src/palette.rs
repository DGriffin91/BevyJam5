use bevy::prelude::*;

// https://lospec.com/palette-list/the-rgb-palette
pub const RGB_PALETTE: [[Color; 7]; 3] = [
    [
        Color::Srgba(Srgba {
            red: 0.263,
            green: 0.153,
            blue: 0.310,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.380,
            green: 0.200,
            blue: 0.337,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.522,
            green: 0.255,
            blue: 0.325,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.631,
            green: 0.349,
            blue: 0.333,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.729,
            green: 0.537,
            blue: 0.447,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.788,
            green: 0.702,
            blue: 0.561,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.961,
            green: 0.929,
            blue: 0.863,
            alpha: 1.0,
        }),
    ],
    [
        Color::Srgba(Srgba {
            red: 0.204,
            green: 0.286,
            blue: 0.361,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.204,
            green: 0.365,
            blue: 0.412,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.208,
            green: 0.478,
            blue: 0.459,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.271,
            green: 0.588,
            blue: 0.471,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.380,
            green: 0.702,
            blue: 0.427,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.569,
            green: 0.769,
            blue: 0.545,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.769,
            green: 0.871,
            blue: 0.741,
            alpha: 1.0,
        }),
    ],
    [
        Color::Srgba(Srgba {
            red: 0.200,
            green: 0.086,
            blue: 0.431,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.133,
            green: 0.180,
            blue: 0.612,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.192,
            green: 0.357,
            blue: 0.741,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.314,
            green: 0.506,
            blue: 0.820,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.435,
            green: 0.678,
            blue: 0.890,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.549,
            green: 0.784,
            blue: 0.812,
            alpha: 1.0,
        }),
        Color::Srgba(Srgba {
            red: 0.714,
            green: 0.831,
            blue: 0.792,
            alpha: 1.0,
        }),
    ],
];
