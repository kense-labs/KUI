//! Resolution-independent line icons and reversible stroke interpolation.
use gpui::{Hsla, PathBuilder, canvas, point, prelude::*, px};
use std::sync::LazyLock;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    More,
    Arrow,
    Send,
    Down,
    Up,
    Check,
    Close,
    Menu,
    Play,
    Pause,
    Plus,
    Minus,
    Copy,
    Link,
    Archive,
    Undo,
    Redo,
    Comment,
    Search,
    Slash,
    ZoomIn,
    ZoomOut,
    Fit,
    Lock,
    Eye,
    Pin,
    Trash,
    Clock,
    Download,
    Edit,
    Globe,
    Save,
    Sun,
    Moon,
    Bell,
    BellOff,
    Heart,
    User,
    Folder,
    FolderKanban,
    ClosedFolder,
    FolderCheck,
    HelpCircle,
    FileText,
    SmilePlus,
    Celebrate,
    Like,
    Bulb,
    Curious,
    Rocket,
    Monitor,
    Settings,
    Card,
    LogOut,
    Spinner,
    Alert,
    Book,
}
type Stroke = &'static [(f32, f32)];

// Both zoom marks sit at the centre of this 7-unit lens on the 24-unit grid.
const ZOOM_JOINT: (f32, f32) = (14.95, 14.95);
const ZOOM_LENS: Stroke = &[
    (17., 10.),
    (16.47, 12.68),
    ZOOM_JOINT,
    (12.68, 16.47),
    (10., 17.),
    (7.32, 16.47),
    (5.05, 14.95),
    (3.53, 12.68),
    (3., 10.),
    (3.53, 7.32),
    (5.05, 5.05),
    (7.32, 3.53),
    (10., 3.),
    (12.68, 3.53),
    (14.95, 5.05),
    (16.47, 7.32),
    (17., 10.),
];
const ZOOM_HANDLE: Stroke = &[ZOOM_JOINT, (22., 22.)];
const ZOOM_BAR: Stroke = &[(7., 10.), (13., 10.)];

const FOLDER_OUTLINE: Stroke = &[
    (4., 4.),
    (8.5, 4.),
    (9.5, 4.4),
    (11.5, 7.),
    (20., 7.),
    (21.4, 7.6),
    (22., 9.),
    (22., 19.),
    (21.4, 20.4),
    (20., 21.),
    (4., 21.),
    (2.6, 20.4),
    (2., 19.),
    (2., 6.),
    (2.6, 4.6),
    (4., 4.),
];

impl Glyph {
    fn strokes(self) -> &'static [Stroke] {
        match self {
            Self::Like => &[
                &[
                    (3.0, 10.0),
                    (7.0, 10.0),
                    (7.0, 21.0),
                    (3.0, 21.0),
                    (3.0, 10.0),
                ],
                &[
                    (7.0, 11.0),
                    (11.0, 7.0),
                    (12.0, 3.0),
                    (14.0, 3.0),
                    (15.0, 5.0),
                    (14.0, 10.0),
                    (20.0, 10.0),
                    (21.0, 12.0),
                    (19.0, 20.0),
                    (17.0, 21.0),
                    (11.0, 21.0),
                    (7.0, 19.0),
                ],
            ],
            Self::Celebrate => &[
                &[(3.0, 21.0), (7.0, 9.0), (15.0, 17.0), (3.0, 21.0)],
                &[(6.0, 14.0), (10.0, 18.0)],
                &[(10.0, 6.0), (12.0, 3.0)],
                &[(15.0, 10.0), (18.0, 7.0), (20.0, 7.0)],
                &[(16.0, 14.0), (21.0, 13.0)],
                &[(15.0, 3.0), (16.0, 4.0), (15.0, 7.0)],
                &[(20.0, 3.0), (21.0, 3.0)],
            ],
            Self::Bulb => &[
                &[
                    (9.0, 17.0),
                    (8.0, 15.0),
                    (6.0, 12.0),
                    (5.5, 9.0),
                    (6.0, 6.0),
                    (8.5, 3.5),
                    (12.0, 2.5),
                    (15.5, 3.5),
                    (18.0, 6.0),
                    (18.5, 9.0),
                    (18.0, 12.0),
                    (16.0, 15.0),
                    (15.0, 17.0),
                    (9.0, 17.0),
                ],
                &[(9.0, 19.0), (15.0, 19.0)],
                &[(10.0, 21.0), (14.0, 21.0)],
            ],
            Self::Curious => &[
                &[
                    (7.0, 5.0),
                    (10.0, 3.0),
                    (14.0, 3.0),
                    (17.0, 5.0),
                    (18.0, 8.0),
                    (17.0, 10.0),
                    (13.0, 13.0),
                    (12.0, 15.0),
                ],
                &[(12.0, 18.0), (12.0, 19.0)],
                &[
                    (4.0, 5.0),
                    (2.0, 9.0),
                    (2.0, 14.0),
                    (4.0, 18.0),
                    (8.0, 21.0),
                    (13.0, 22.0),
                    (18.0, 20.0),
                    (21.0, 16.0),
                    (22.0, 11.0),
                    (21.0, 8.0),
                ],
            ],
            Self::Rocket => &[
                &[
                    (5.0, 14.0),
                    (8.0, 9.0),
                    (13.0, 5.0),
                    (17.0, 3.0),
                    (21.0, 3.0),
                    (21.0, 7.0),
                    (19.0, 12.0),
                    (14.0, 17.0),
                    (10.0, 19.0),
                    (5.0, 14.0),
                ],
                &[(7.0, 10.0), (4.0, 10.0), (2.0, 14.0), (5.0, 14.0)],
                &[(14.0, 16.0), (14.0, 20.0), (10.0, 22.0), (10.0, 19.0)],
                &[(6.0, 18.0), (3.0, 21.0)],
                &[
                    (14.0, 7.0),
                    (16.0, 7.0),
                    (17.0, 9.0),
                    (16.0, 10.0),
                    (14.0, 10.0),
                    (13.0, 9.0),
                    (14.0, 7.0),
                ],
            ],
            Self::SmilePlus => &[
                &[
                    (19.0, 11.0),
                    (19.0, 14.0),
                    (17.0, 18.0),
                    (13.0, 20.0),
                    (9.0, 20.0),
                    (5.0, 18.0),
                    (3.0, 14.0),
                    (3.0, 10.0),
                    (5.0, 6.0),
                    (9.0, 4.0),
                    (12.0, 4.0),
                ],
                &[(7.0, 10.0), (8.0, 10.0)],
                &[(13.0, 10.0), (14.0, 10.0)],
                &[(7.0, 14.0), (9.0, 16.0), (12.0, 16.0), (14.0, 14.0)],
                &[(18.0, 2.0), (18.0, 8.0)],
                &[(15.0, 5.0), (21.0, 5.0)],
            ],
            Self::Monitor => &[
                &[
                    (3.0, 4.0),
                    (21.0, 4.0),
                    (21.0, 16.0),
                    (3.0, 16.0),
                    (3.0, 4.0),
                ],
                &[(12.0, 16.0), (12.0, 20.0)],
                &[(8.0, 21.0), (16.0, 21.0)],
            ],
            Self::Settings => &[
                &[
                    (9.0, 3.0),
                    (14.0, 3.0),
                    (15.0, 6.0),
                    (18.0, 6.0),
                    (20.0, 9.0),
                    (18.5, 12.0),
                    (20.0, 15.0),
                    (18.0, 18.0),
                    (15.0, 18.0),
                    (14.0, 21.0),
                    (9.0, 21.0),
                    (8.0, 18.0),
                    (5.0, 18.0),
                    (3.0, 15.0),
                    (5.0, 12.0),
                    (3.0, 9.0),
                    (5.0, 6.0),
                    (8.0, 6.0),
                    (9.0, 3.0),
                ],
                &[
                    (9.0, 11.0),
                    (10.0, 9.0),
                    (13.0, 9.0),
                    (15.0, 11.0),
                    (15.0, 13.0),
                    (13.0, 15.0),
                    (10.0, 15.0),
                    (9.0, 13.0),
                    (9.0, 11.0),
                ],
            ],
            Self::Card => &[
                &[
                    (2.0, 5.0),
                    (22.0, 5.0),
                    (22.0, 19.0),
                    (2.0, 19.0),
                    (2.0, 5.0),
                ],
                &[(2.0, 10.0), (22.0, 10.0)],
                &[(6.0, 15.0), (10.0, 15.0)],
            ],
            Self::LogOut => &[
                &[(9.0, 3.0), (4.0, 3.0), (4.0, 21.0), (9.0, 21.0)],
                &[(10.0, 12.0), (21.0, 12.0)],
                &[(16.0, 7.0), (21.0, 12.0), (16.0, 17.0)],
            ],
            Self::More => &[
                &[(4., 12.), (6., 12.)],
                &[(11., 12.), (13., 12.)],
                &[(18., 12.), (20., 12.)],
            ],
            Self::Arrow => &[
                &[(5.00, 12.00), (19.00, 12.00)],
                &[(13.00, 6.00), (19.00, 12.00), (13.00, 18.00)],
            ],
            Self::Send => &[
                &[
                    (3.00, 10.00),
                    (21.00, 3.00),
                    (16.00, 21.00),
                    (11.00, 14.00),
                    (3.00, 10.00),
                ],
                &[(11.00, 14.00), (21.00, 3.00)],
            ],
            Self::Down => &[&[(5.00, 9.00), (12.00, 16.00), (19.00, 9.00)]],
            Self::Up => &[&[(5.00, 15.00), (12.00, 8.00), (19.00, 15.00)]],
            Self::Check => &[&[(3.00, 12.00), (9.00, 18.00), (21.00, 5.00)]],
            Self::Close => &[
                &[(5.00, 5.00), (19.00, 19.00)],
                &[(5.00, 19.00), (19.00, 5.00)],
            ],
            Self::Menu => &[
                &[(3.00, 6.00), (21.00, 6.00)],
                &[(3.00, 12.00), (21.00, 12.00)],
                &[(3.00, 18.00), (21.00, 18.00)],
            ],
            Self::Play => &[&[(6.00, 3.00), (21.00, 12.00), (6.00, 21.00), (6.00, 3.00)]],
            Self::Pause => &[
                &[(8.00, 4.00), (8.00, 20.00)],
                &[(16.00, 4.00), (16.00, 20.00)],
            ],
            Self::Plus => &[
                &[(4.00, 12.00), (20.00, 12.00)],
                &[(12.00, 4.00), (12.00, 20.00)],
            ],
            Self::Minus => &[&[(4.00, 12.00), (20.00, 12.00)]],
            Self::Copy => &[
                &[
                    (8.00, 7.00),
                    (21.00, 7.00),
                    (21.00, 21.00),
                    (8.00, 21.00),
                    (8.00, 7.00),
                ],
                &[
                    (4.00, 16.00),
                    (3.00, 16.00),
                    (3.00, 3.00),
                    (16.00, 3.00),
                    (16.00, 4.00),
                ],
            ],
            Self::Link => &[
                &[
                    (10.00, 15.00),
                    (8.00, 17.00),
                    (6.00, 18.00),
                    (4.00, 17.00),
                    (3.00, 15.00),
                    (3.00, 13.00),
                    (8.00, 8.00),
                    (10.00, 8.00),
                ],
                &[
                    (14.00, 9.00),
                    (16.00, 7.00),
                    (18.00, 6.00),
                    (20.00, 7.00),
                    (21.00, 9.00),
                    (21.00, 11.00),
                    (16.00, 16.00),
                    (14.00, 16.00),
                ],
                &[(8.00, 15.00), (16.00, 8.00)],
            ],
            Self::Archive => &[
                &[
                    (3.00, 4.00),
                    (21.00, 4.00),
                    (21.00, 9.00),
                    (3.00, 9.00),
                    (3.00, 4.00),
                ],
                &[(5.00, 10.00), (5.00, 21.00), (19.00, 21.00), (19.00, 10.00)],
                &[(10.00, 12.00), (14.00, 12.00)],
            ],
            Self::Undo => &[
                &[(3.00, 5.00), (3.00, 11.00), (9.00, 11.00)],
                &[
                    (3.00, 11.00),
                    (7.00, 7.00),
                    (12.00, 6.00),
                    (16.00, 7.00),
                    (19.00, 10.00),
                    (20.00, 14.00),
                    (19.00, 17.00),
                ],
            ],
            Self::Redo => &[
                &[(21.00, 5.00), (21.00, 11.00), (15.00, 11.00)],
                &[
                    (21.00, 11.00),
                    (17.00, 7.00),
                    (12.00, 6.00),
                    (8.00, 7.00),
                    (5.00, 10.00),
                    (4.00, 14.00),
                    (5.00, 17.00),
                ],
            ],
            Self::Comment => &[&[
                (3.00, 4.00),
                (21.00, 4.00),
                (21.00, 17.00),
                (10.00, 17.00),
                (4.00, 21.00),
                (4.00, 17.00),
                (3.00, 17.00),
                (3.00, 4.00),
            ]],
            Self::Search => &[ZOOM_LENS, ZOOM_HANDLE],
            Self::Slash => &[&[(3., 3.), (21., 21.)]],
            Self::ZoomIn => &[ZOOM_LENS, ZOOM_HANDLE, ZOOM_BAR, &[(10., 7.), (10., 13.)]],
            Self::ZoomOut => &[ZOOM_LENS, ZOOM_HANDLE, ZOOM_BAR],
            Self::Fit => &[
                &[(3.00, 8.00), (3.00, 3.00), (8.00, 3.00)],
                &[(16.00, 3.00), (21.00, 3.00), (21.00, 8.00)],
                &[(21.00, 16.00), (21.00, 21.00), (16.00, 21.00)],
                &[(8.00, 21.00), (3.00, 21.00), (3.00, 16.00)],
            ],
            Self::Lock => &[
                &[
                    (4.00, 10.00),
                    (20.00, 10.00),
                    (20.00, 21.00),
                    (4.00, 21.00),
                    (4.00, 10.00),
                ],
                &[
                    (7.00, 10.00),
                    (7.00, 7.00),
                    (8.00, 4.00),
                    (11.00, 3.00),
                    (14.00, 3.00),
                    (17.00, 5.00),
                    (17.00, 10.00),
                ],
            ],
            Self::Eye => {
                // Use the same 40 intervals as the stroke renderer, with a closed seam at the right.
                static RINGS: LazyLock<[[(f32, f32); 41]; 2]> = LazyLock::new(|| {
                    [(10., 6.), (3., 3.)].map(|(rx, ry)| {
                        std::array::from_fn(|i| {
                            let angle = std::f32::consts::TAU * (i % 40) as f32 / 40.;
                            let (sin, cos) = angle.sin_cos();
                            (12. + rx * cos, 12. + ry * sin)
                        })
                    })
                });
                static STROKES: LazyLock<[Stroke; 2]> = LazyLock::new(|| [&RINGS[0], &RINGS[1]]);
                &*STROKES
            }
            Self::Pin => &[
                &[
                    (8.00, 3.00),
                    (19.00, 3.00),
                    (17.00, 6.00),
                    (17.00, 11.00),
                    (20.00, 15.00),
                    (6.00, 15.00),
                    (9.00, 10.00),
                    (9.00, 6.00),
                    (8.00, 3.00),
                ],
                &[(13.00, 15.00), (10.00, 22.00)],
            ],
            Self::Trash => &[
                &[(3.00, 6.00), (21.00, 6.00)],
                &[(8.00, 6.00), (9.00, 3.00), (15.00, 3.00), (16.00, 6.00)],
                &[(5.00, 7.00), (6.00, 21.00), (18.00, 21.00), (19.00, 7.00)],
                &[(10.00, 10.00), (10.00, 17.00)],
                &[(14.00, 10.00), (14.00, 17.00)],
            ],
            Self::Clock => &[
                &[
                    (21.00, 12.00),
                    (20.31, 15.44),
                    (18.36, 18.36),
                    (15.44, 20.31),
                    (12.00, 21.00),
                    (8.56, 20.31),
                    (5.64, 18.36),
                    (3.69, 15.44),
                    (3.00, 12.00),
                    (3.69, 8.56),
                    (5.64, 5.64),
                    (8.56, 3.69),
                    (12.00, 3.00),
                    (15.44, 3.69),
                    (18.36, 5.64),
                    (20.31, 8.56),
                    (21.00, 12.00),
                ],
                &[(12.00, 6.00), (12.00, 12.00), (16.00, 14.00)],
            ],
            Self::Download => &[
                &[(12.00, 3.00), (12.00, 15.00)],
                &[(7.00, 10.00), (12.00, 15.00), (17.00, 10.00)],
                &[(3.00, 16.00), (3.00, 21.00), (21.00, 21.00), (21.00, 16.00)],
            ],
            Self::Edit => &[
                &[
                    (4.00, 16.00),
                    (16.00, 4.00),
                    (20.00, 8.00),
                    (8.00, 20.00),
                    (3.00, 21.00),
                    (4.00, 16.00),
                ],
                &[(13.00, 7.00), (17.00, 11.00)],
            ],
            Self::Globe => &[
                &[
                    (21.00, 12.00),
                    (20.31, 15.44),
                    (18.36, 18.36),
                    (15.44, 20.31),
                    (12.00, 21.00),
                    (8.56, 20.31),
                    (5.64, 18.36),
                    (3.69, 15.44),
                    (3.00, 12.00),
                    (3.69, 8.56),
                    (5.64, 5.64),
                    (8.56, 3.69),
                    (12.00, 3.00),
                    (15.44, 3.69),
                    (18.36, 5.64),
                    (20.31, 8.56),
                    (21.00, 12.00),
                ],
                &[(3.00, 12.00), (21.00, 12.00)],
                &[
                    (12.00, 3.00),
                    (8.00, 7.00),
                    (7.00, 12.00),
                    (8.00, 17.00),
                    (12.00, 21.00),
                    (16.00, 17.00),
                    (17.00, 12.00),
                    (16.00, 7.00),
                    (12.00, 3.00),
                ],
            ],
            Self::Save => &[
                &[
                    (3.00, 3.00),
                    (17.00, 3.00),
                    (21.00, 7.00),
                    (21.00, 21.00),
                    (3.00, 21.00),
                    (3.00, 3.00),
                ],
                &[(7.00, 4.00), (7.00, 9.00), (16.00, 9.00), (16.00, 4.00)],
                &[(7.00, 20.00), (7.00, 14.00), (17.00, 14.00), (17.00, 20.00)],
            ],
            Self::Sun => &[
                &[
                    (17.00, 12.00),
                    (16.62, 13.91),
                    (15.54, 15.54),
                    (13.91, 16.62),
                    (12.00, 17.00),
                    (10.09, 16.62),
                    (8.46, 15.54),
                    (7.38, 13.91),
                    (7.00, 12.00),
                    (7.38, 10.09),
                    (8.46, 8.46),
                    (10.09, 7.38),
                    (12.00, 7.00),
                    (13.91, 7.38),
                    (15.54, 8.46),
                    (16.62, 10.09),
                    (17.00, 12.00),
                ],
                &[(12.00, 1.00), (12.00, 4.00)],
                &[(12.00, 20.00), (12.00, 23.00)],
                &[(1.00, 12.00), (4.00, 12.00)],
                &[(20.00, 12.00), (23.00, 12.00)],
                &[(4.00, 4.00), (6.00, 6.00)],
                &[(18.00, 18.00), (20.00, 20.00)],
                &[(4.00, 20.00), (6.00, 18.00)],
                &[(18.00, 6.00), (20.00, 4.00)],
            ],
            Self::Moon => &[&[
                (17.00, 16.00),
                (20.00, 15.00),
                (22.00, 13.00),
                (21.00, 17.00),
                (18.00, 20.00),
                (14.00, 22.00),
                (10.00, 22.00),
                (6.00, 20.00),
                (3.00, 17.00),
                (2.00, 12.00),
                (3.00, 8.00),
                (6.00, 4.00),
                (10.00, 2.00),
                (9.00, 5.00),
                (9.00, 9.00),
                (11.00, 13.00),
                (14.00, 15.00),
                (17.00, 16.00),
            ]],
            Self::Bell => &[
                &[
                    (3.00, 18.00),
                    (5.00, 15.00),
                    (6.00, 8.00),
                    (8.00, 5.00),
                    (12.00, 4.00),
                    (16.00, 5.00),
                    (18.00, 8.00),
                    (19.00, 15.00),
                    (21.00, 18.00),
                    (3.00, 18.00),
                ],
                &[(10.00, 21.00), (12.00, 22.00), (14.00, 21.00)],
            ],
            Self::BellOff => &[
                &[(3.00, 18.00), (5.00, 15.00), (6.00, 8.00)],
                &[(10.00, 4.00), (14.00, 4.00), (17.00, 7.00), (19.00, 15.00)],
                &[(21.00, 18.00), (3.00, 18.00)],
                &[(10.00, 21.00), (12.00, 22.00), (14.00, 21.00)],
                &[(2.00, 2.00), (22.00, 22.00)],
            ],
            Self::Heart => &[&[
                (12.00, 21.00),
                (3.00, 12.00),
                (2.00, 8.00),
                (3.00, 5.00),
                (6.00, 3.00),
                (9.00, 3.00),
                (12.00, 6.00),
                (15.00, 3.00),
                (18.00, 3.00),
                (21.00, 5.00),
                (22.00, 8.00),
                (21.00, 12.00),
                (12.00, 21.00),
            ]],
            Self::User => &[
                &[
                    (13.00, 7.00),
                    (12.70, 8.53),
                    (11.83, 9.83),
                    (10.53, 10.70),
                    (9.00, 11.00),
                    (7.47, 10.70),
                    (6.17, 9.83),
                    (5.30, 8.53),
                    (5.00, 7.00),
                    (5.30, 5.47),
                    (6.17, 4.17),
                    (7.47, 3.30),
                    (9.00, 3.00),
                    (10.53, 3.30),
                    (11.83, 4.17),
                    (12.70, 5.47),
                    (13.00, 7.00),
                ],
                &[
                    (2.00, 21.00),
                    (3.00, 17.00),
                    (6.00, 14.00),
                    (12.00, 14.00),
                    (15.00, 17.00),
                ],
                &[(19.00, 9.00), (19.00, 15.00)],
                &[(16.00, 12.00), (22.00, 12.00)],
            ],
            Self::Folder => &[&[
                (2.00, 6.00),
                (9.00, 6.00),
                (11.00, 8.00),
                (22.00, 8.00),
                (20.00, 20.00),
                (3.00, 20.00),
                (2.00, 6.00),
            ]],
            Self::ClosedFolder => &[FOLDER_OUTLINE],
            Self::FolderCheck => &[FOLDER_OUTLINE, &[(8., 14.), (10.8, 16.8), (16.4, 11.1)]],
            Self::HelpCircle => &[
                &[
                    (12., 2.),
                    (15.8, 2.8),
                    (19.1, 4.9),
                    (21.2, 8.2),
                    (22., 12.),
                    (21.2, 15.8),
                    (19.1, 19.1),
                    (15.8, 21.2),
                    (12., 22.),
                    (8.2, 21.2),
                    (4.9, 19.1),
                    (2.8, 15.8),
                    (2., 12.),
                    (2.8, 8.2),
                    (4.9, 4.9),
                    (8.2, 2.8),
                    (12., 2.),
                ],
                &[
                    (9.1, 9.3),
                    (9.7, 7.8),
                    (11., 7.1),
                    (13., 7.2),
                    (14.4, 8.1),
                    (14.9, 9.4),
                    (14.4, 10.9),
                    (12.6, 12.1),
                    (12., 13.3),
                ],
                &[(12., 16.4), (12., 16.9)],
            ],
            Self::FolderKanban => &[
                FOLDER_OUTLINE,
                &[(8., 10.5), (8., 15.5)],
                &[(12., 10.5), (12., 13.)],
                &[(16., 10.5), (16., 17.)],
            ],
            Self::FileText => &[
                &[
                    (6., 2.),
                    (14., 2.),
                    (20., 8.),
                    (20., 20.),
                    (19.5, 21.4),
                    (18., 22.),
                    (6., 22.),
                    (4.6, 21.5),
                    (4., 20.),
                    (4., 4.),
                    (4.5, 2.6),
                    (6., 2.),
                ],
                &[(14., 2.5), (14., 7.), (14.3, 8.), (15.3, 8.5), (19.5, 8.5)],
                &[(7.5, 8.5), (10.5, 8.5)],
                &[(7.5, 12.5), (16.5, 12.5)],
                &[(7.5, 16.5), (16.5, 16.5)],
            ],
            Self::Spinner => &[&[
                (12.00, 3.00),
                (16.00, 4.00),
                (19.00, 7.00),
                (21.00, 11.00),
                (21.00, 14.00),
            ]],
            Self::Alert => &[
                &[(12.00, 4.00), (12.00, 14.00)],
                &[(12.00, 19.00), (12.00, 20.00)],
            ],
            Self::Book => &[&[
                (3.00, 4.00),
                (10.00, 4.00),
                (12.00, 6.00),
                (14.00, 4.00),
                (21.00, 4.00),
                (21.00, 19.00),
                (14.00, 19.00),
                (12.00, 21.00),
                (10.00, 19.00),
                (3.00, 19.00),
                (3.00, 4.00),
            ]],
        }
    }
}
fn sample(line: Stroke, t: f32) -> (f32, f32) {
    if line.is_empty() {
        return (12., 12.);
    }
    let index = t * (line.len() - 1) as f32;
    let a = line[index.floor() as usize];
    let b = line[(index.ceil() as usize).min(line.len() - 1)];
    let f = index.fract();
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}
pub fn drawing(a: Glyph, b: Glyph, t: f32, side: f32, ink: Hsla, filled: bool) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let t = t.clamp(0., 1.);
            let (a, b) = (a.strokes(), b.strokes());
            for i in 0..a.len().max(b.len()) {
                let x = a.get(i).copied().unwrap_or(&[]);
                let y = b.get(i).copied().unwrap_or(&[]);
                let mut path = if filled && i == 0 {
                    PathBuilder::fill()
                } else {
                    PathBuilder::stroke(px((side / 14.).max(1.25)))
                };
                for j in 0..=40 {
                    let u = j as f32 / 40.;
                    let (x1, y1) = sample(x, u);
                    let (x2, y2) = sample(y, u);
                    let pt = point(
                        bounds.origin.x + px((x1 + (x2 - x1) * t) * side / 24.),
                        bounds.origin.y + px((y1 + (y2 - y1) * t) * side / 24.),
                    );
                    if j == 0 {
                        path.move_to(pt)
                    } else {
                        path.line_to(pt)
                    }
                }
                if filled && i == 0 {
                    path.close()
                }
                if let Ok(p) = path.build() {
                    w.paint_path(p, ink);
                }
            }
        },
    )
    .size(px(side))
    .flex_none()
}
pub fn icon(g: Glyph, side: f32, ink: Hsla) -> impl IntoElement {
    drawing(g, g, 0., side, ink, false)
}

/// Rotating, open circular stroke. A zero turn is also the reduced-motion frame.
pub fn spinner(turn: f32, side: f32, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let mut path = PathBuilder::stroke(px(1.6));
            for i in 0..32 {
                let a = (turn + i as f32 / 42.) * std::f32::consts::TAU;
                let q = point(
                    bounds.origin.x + px(side * (0.5 + 0.36 * a.cos())),
                    bounds.origin.y + px(side * (0.5 + 0.36 * a.sin())),
                );
                if i == 0 {
                    path.move_to(q)
                } else {
                    path.line_to(q)
                }
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(side))
    .flex_none()
}

/// Draw a mark with independent rotation, optical scale, opacity and stroke reveal.
pub fn transformed(
    g: Glyph,
    side: f32,
    ink: Hsla,
    angle: f32,
    scale: f32,
    offset: (f32, f32),
    reveal: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut pen = Pen {
                w,
                origin: b.origin,
                side,
            };
            for s in g.strokes() {
                let pts = s
                    .iter()
                    .map(|&(x, y)| {
                        let (x, y) = rotate((x, y), (12., 12.), angle);
                        (
                            12. + (x - 12.) * scale + offset.0,
                            12. + (y - 12.) * scale + offset.1,
                        )
                    })
                    .collect::<Vec<_>>();
                pen.trace(&pts, ink, 1.75 * scale, reveal, false)
            }
        },
    )
    .size(px(side))
    .flex_none()
}
fn rotate((x, y): (f32, f32), (cx, cy): (f32, f32), a: f32) -> (f32, f32) {
    let (s, c) = a.sin_cos();
    (
        cx + (x - cx) * c - (y - cy) * s,
        cy + (x - cx) * s + (y - cy) * c,
    )
}
struct Pen<'w> {
    w: &'w mut gpui::Window,
    origin: gpui::Point<gpui::Pixels>,
    side: f32,
}
impl Pen<'_> {
    fn trace(&mut self, points: &[(f32, f32)], ink: Hsla, weight: f32, progress: f32, fill: bool) {
        if points.len() < 2 || progress <= 0. || ink.a <= 0.001 {
            return;
        }
        let convert = |p: (f32, f32)| {
            point(
                self.origin.x + px(p.0 * self.side / 24.),
                self.origin.y + px(p.1 * self.side / 24.),
            )
        };
        let mut path = if fill {
            PathBuilder::fill()
        } else {
            PathBuilder::stroke(px(weight * self.side / 24.))
        };
        path.move_to(convert(points[0]));
        let mut left = if progress >= 1. {
            f32::INFINITY
        } else {
            points
                .windows(2)
                .map(|p| (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1))
                .sum::<f32>()
                * progress
        };
        for p in points.windows(2) {
            let (x, y) = p[0];
            let (dx, dy) = (p[1].0 - x, p[1].1 - y);
            let len = dx.hypot(dy);
            if left < len && len > 0. {
                path.line_to(convert((x + dx * left / len, y + dy * left / len)));
                break;
            }
            path.line_to(convert(p[1]));
            left -= len;
        }
        if fill {
            path.close()
        }
        if let Ok(p) = path.build() {
            self.w.paint_path(p, ink)
        }
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, c: Hsla, weight: f32, fill: bool) {
        let mut pts = [(0., 0.); 49];
        for (j, p) in pts.iter_mut().enumerate() {
            let a = j as f32 * std::f32::consts::TAU / 48.;
            *p = (x + r * a.cos(), y + r * a.sin())
        }
        self.trace(&pts, c, weight, 1., fill)
    }
}
/// Sixteen-pixel countdown clock; the stroke drains clockwise from twelve.
pub fn countdown_ring(remaining: f32, side: f32, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut pen = Pen {
                w,
                origin: b.origin,
                side,
            };
            let pts = std::array::from_fn::<_, 65, _>(|i| {
                let angle = std::f32::consts::TAU * (i as f32 / 64. - 0.25);
                (12. + 9.375 * angle.cos(), 12. + 9.375 * angle.sin())
            });
            pen.trace(&pts, ink.opacity(0.22), 2.625, 1., false);
            pen.trace(&pts, ink, 2.625, remaining.clamp(0., 1.), false);
        },
    )
    .size(px(side))
    .flex_none()
}
/// The eight two-state marks have different geometry. Only bell and heart carry a one-shot clock.
pub fn morph_mark(
    pair: [Glyph; 2],
    t: f32,
    age: f32,
    side: f32,
    ink: Hsla,
    accent: Hsla,
    surface: Hsla,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut p = Pen {
                w,
                origin: b.origin,
                side,
            };
            let t = t.clamp(0., 1.);
            let mix = |a: f32, b: f32| a + (b - a) * t;
            let line = |p: &mut Pen<'_>, a, b| p.trace(&[a, b], ink, 1.75, 1., false);
            match pair {
                [Glyph::Menu, Glyph::Close] => {
                    for j in [-1., 1.] {
                        let y = 12. + j * 5. * (1. - t);
                        let angle = -j * t * std::f32::consts::FRAC_PI_4;
                        line(
                            &mut p,
                            rotate((4., y), (12., y), angle),
                            rotate((20., y), (12., y), angle),
                        )
                    }
                    p.trace(
                        &[(mix(4., 12.), 12.), (mix(20., 12.), 12.)],
                        ink.opacity(1. - t),
                        1.75,
                        1.,
                        false,
                    );
                }
                [Glyph::Play, Glyph::Pause] => {
                    for (a, b) in [
                        (
                            [(5., 3.), (11.5, 6.7), (11.5, 17.3), (5., 21.)],
                            [(5., 4.), (10., 4.), (10., 20.), (5., 20.)],
                        ),
                        (
                            [(11.5, 6.7), (21., 12.), (21., 12.), (11.5, 17.3)],
                            [(14., 4.), (19., 4.), (19., 20.), (14., 20.)],
                        ),
                    ] {
                        let pts: Vec<_> = a
                            .into_iter()
                            .zip(b)
                            .map(|(a, b)| (mix(a.0, b.0), mix(a.1, b.1)))
                            .collect();
                        p.trace(&pts, ink, 1., 1., true)
                    }
                }
                [Glyph::Plus, Glyph::Check] => {
                    line(&mut p, (mix(4., 3.), 12.), (mix(20., 8.8), mix(12., 17.8)));
                    line(
                        &mut p,
                        (mix(12., 20.8), mix(4., 5.8)),
                        (mix(12., 8.8), mix(20., 17.8)),
                    )
                }
                [Glyph::Copy, Glyph::Check] => {
                    for s in Glyph::Copy.strokes() {
                        let pts: Vec<_> = s
                            .iter()
                            .map(|&(x, y)| {
                                (
                                    12. + (x - 12.) * (1. - 0.3 * t),
                                    12. + (y - 12.) * (1. - 0.3 * t),
                                )
                            })
                            .collect();
                        p.trace(&pts, ink.opacity((1. - 1.8 * t).max(0.)), 1.75, 1., false)
                    }
                    p.trace(
                        Glyph::Check.strokes()[0],
                        ink,
                        1.85,
                        ((t - 0.13) / 0.87).clamp(0., 1.),
                        false,
                    );
                }
                [Glyph::Down, Glyph::Up] => {
                    let pts: Vec<_> = Glyph::Down.strokes()[0]
                        .iter()
                        .map(|&v| rotate(v, (12., 12.), t * std::f32::consts::PI))
                        .collect();
                    p.trace(&pts, ink, 1.75, 1., false)
                }
                [Glyph::Sun, Glyph::Moon] => {
                    let r = mix(4., 8.6);
                    p.circle(12., 12., r, ink, 1.75, false);
                    for i in 0..8 {
                        let a = i as f32 * std::f32::consts::FRAC_PI_4;
                        let a1 = rotate((12., mix(2., 5.)), (12., 12.), a);
                        let b1 = rotate((12., 5.), (12., 12.), a);
                        p.trace(&[a1, b1], ink.opacity(1. - t), 1.75, 1., false)
                    }
                    // Intersect the two circular outlines; the crescent has no exposed mask edge.
                    if t > 0.0001 {
                        let d = mix(17.2, 8.6);
                        let q = (d / (2. * r)).clamp(-1., 1.);
                        if q < 1. {
                            let center = (
                                12. + d * std::f32::consts::FRAC_1_SQRT_2,
                                12. - d * std::f32::consts::FRAC_1_SQRT_2,
                            );
                            p.circle(center.0, center.1, r + 0.7, surface, 0., true);
                            let mid = std::f32::consts::PI * 0.75;
                            let half = q.acos();
                            let pts: Vec<_> = (0..=32)
                                .map(|i| {
                                    let a = mid - half + 2. * half * i as f32 / 32.;
                                    (center.0 + r * a.cos(), center.1 + r * a.sin())
                                })
                                .collect();
                            p.trace(&pts, ink, 1.75, 1., false)
                        }
                    }
                }
                [Glyph::Bell, Glyph::BellOff] => {
                    let sway = |age: f32| {
                        if (0.0..0.9).contains(&age) {
                            (age * 31.).sin() * (-4. * age).exp() * 0.34
                        } else {
                            0.
                        }
                    };
                    for (j, s) in Glyph::Bell.strokes().iter().enumerate() {
                        let a = if j == 0 {
                            sway(age)
                        } else {
                            sway(age - 0.04) * 1.35
                        };
                        let pts: Vec<_> = s.iter().map(|&v| rotate(v, (12., 3.), a)).collect();
                        p.trace(&pts, ink, 1.75, 1., false)
                    }
                    p.trace(&[(3., 3.), (21., 21.)], surface, 4.4, t, false);
                    p.trace(&[(3., 3.), (21., 21.)], ink, 1.75, t, false)
                }
                [Glyph::Heart, Glyph::Heart] => {
                    let pop = if age < 0.45 {
                        1. + 0.21 * (age * 18.).sin() * (-6. * age).exp()
                    } else {
                        1.
                    };
                    let mut pts = Vec::with_capacity(80);
                    for j in 0..=80 {
                        let a = j as f32 * std::f32::consts::TAU / 80.;
                        let x = 16. * a.sin().powi(3);
                        let y = 13. * a.cos()
                            - 5. * (2. * a).cos()
                            - 2. * (3. * a).cos()
                            - (4. * a).cos();
                        pts.push((12. + 0.62 * x * pop, 10.5 - 0.65 * y * pop))
                    }
                    p.trace(&pts, ink.opacity(1. - t), 1.75, 1., false);
                    p.trace(&pts, accent.opacity(t), 1., 1., true);
                    if age < 0.6 {
                        let f = (age / 0.55).clamp(0., 1.);
                        p.circle(
                            12.,
                            12.,
                            5. + 10. * f,
                            accent.opacity(1. - f),
                            1.5 * (1. - f),
                            false,
                        );
                        for j in 0..7 {
                            let a = j as f32 * 0.8976;
                            let r = 8. + 8. * f;
                            p.circle(
                                12. + a.cos() * r,
                                12. + a.sin() * r,
                                (1. - f) * 1.15,
                                accent.opacity(1. - f),
                                1.,
                                true,
                            )
                        }
                    }
                }
                _ => {
                    for (g, k) in [(pair[0], 1. - t), (pair[1], t)] {
                        for s in g.strokes() {
                            p.trace(s, ink.opacity(k), 1.75, 1., false)
                        }
                    }
                }
            }
        },
    )
    .size(px(side))
    .flex_none()
}
