//! Your books: where they are on disk, which page each piece is on, and the page references
//! the lesson plan points to. The books themselves are never copied into the app – PianoCraft
//! opens your own PDFs.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum BookId {
    RussianSchool1a,
    RussianSchool1b,
    ElementsOfMusic,
    NahreScales,
    Nahre251,
    NahreTrills,
    NahreChopin,
    AnnaMagdalena,
}

impl BookId {
    pub const ALL: [BookId; 8] = [
        BookId::RussianSchool1a, BookId::RussianSchool1b, BookId::ElementsOfMusic, BookId::NahreScales,
        BookId::Nahre251, BookId::NahreTrills, BookId::NahreChopin, BookId::AnnaMagdalena,
    ];
    pub fn title(self) -> &'static str {
        match self {
            BookId::RussianSchool1a => "Russian School of Piano Playing – Book 1, Part I (1a)",
            BookId::RussianSchool1b => "Russian School of Piano Playing – Book 1, Part II (1b)",
            BookId::ElementsOfMusic => "Nahre Sol – Elements of Music",
            BookId::NahreScales => "Nahre Sol – Guide to Scales and Modes (exercises)",
            BookId::Nahre251 => "Nahre Sol – Warm-Ups in ii-V-I",
            BookId::NahreTrills => "Nahre Sol – Trills Exercise",
            BookId::NahreChopin => "Nahre Sol – Modified Chopin Etude Exercises",
            BookId::AnnaMagdalena => "Notebook for Anna Magdalena Bach (selections)",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            BookId::RussianSchool1a => "Russian School 1a",
            BookId::RussianSchool1b => "Russian School 1b",
            BookId::ElementsOfMusic => "Elements of Music",
            BookId::NahreScales => "Nahre Sol scales & modes",
            BookId::Nahre251 => "Nahre Sol ii-V-I warm-ups",
            BookId::NahreTrills => "Nahre Sol trills",
            BookId::NahreChopin => "Nahre Sol modified Chopin",
            BookId::AnnaMagdalena => "Anna Magdalena Notebook",
        }
    }
    /// Lower-case fragments that must all appear in the file name.
    fn match_terms(self) -> &'static [&'static [&'static str]] {
        match self {
            BookId::RussianSchool1a => &[&["russian school", "part 1"], &["russian school", "1a"], &["russian_school", "1a"]],
            BookId::RussianSchool1b => &[&["russian school", "1b"], &["russian_school", "1b"], &["russian school", "part 2"]],
            BookId::ElementsOfMusic => &[&["elements", "music", "nahre"], &["elements-of-music"]],
            BookId::NahreScales => &[&["scales-modes"], &["nsg", "scales"], &["scales and modes", "nahre"]],
            BookId::Nahre251 => &[&["251"]],
            BookId::NahreTrills => &[&["trills"]],
            BookId::NahreChopin => &[&["modified", "chopin"]],
            BookId::AnnaMagdalena => &[&["anna magdalena"], &["anna_magdalena"]],
        }
    }
    pub fn matches(self, file_name: &str) -> bool {
        let f = file_name.to_lowercase();
        f.ends_with(".pdf") && self.match_terms().iter().any(|terms| terms.iter().all(|t| f.contains(t)))
    }
}

/// Folders to look in for books: the chosen one first, then Documents, Music, Downloads,
/// Desktop and the usual Google Drive location.
pub fn default_folders(chosen: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if !chosen.trim().is_empty() {
        v.push(PathBuf::from(chosen.trim()));
    }
    v.extend([dirs::document_dir(), dirs::audio_dir(), dirs::download_dir(), dirs::desktop_dir()].into_iter().flatten());
    #[cfg(windows)]
    v.push(PathBuf::from(r"G:\My Drive"));
    v.retain(|p| p.exists());
    v.dedup();
    v
}

/// Find your books inside a folder (searches sub-folders too).
pub fn locate(folder: &Path) -> Vec<(BookId, PathBuf)> {
    let mut found: Vec<(BookId, PathBuf)> = Vec::new();
    fn walk(d: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > 5 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, depth + 1, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")) {
                out.push(p);
            }
        }
    }
    let mut pdfs = Vec::new();
    walk(folder, 0, &mut pdfs);
    for id in BookId::ALL {
        if let Some(p) = pdfs.iter().find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| id.matches(n))) {
            found.push((id, p.clone()));
        }
    }
    found
}

/// (number, PDF page (1-based), title, composer/source)
pub type PieceRef = (u32, usize, &'static str, &'static str);

pub const RS1A: &[PieceRef] = &[
    // I. Singing by ear; the keyboard; notes and the stave
    (1, 8, "Tra-la-la", ""),
    (2, 8, "Guardsmen", ""),
    (3, 8, "Counting Magpies", ""),
    (4, 8, "Ding-dong", ""),
    (5, 8, "Do Not Fly, Nightingale", ""),
    (6, 9, "Corn-flower", ""),
    (7, 9, "We Sowed Wheat", ""),
    (8, 11, "Rabbit in the Garden", ""),
    (9, 11, "The Goat with Horns", ""),
    (10, 11, "Cuckoo", ""),
    (11, 11, "Hullo!", ""),
    (12, 11, "In the Shadow of the Hill", ""),
    (13, 14, "The Green Meadow", ""),
    (14, 14, "Cockerel", ""),
    // II. Melodies played with different fingers (non legato)
    (15, 15, "Rabbit", ""),
    (16, 15, "Rain", ""),
    (17, 15, "Study", "E. Gnesina"),
    (18, 16, "On the Ice", "M. Krasyev"),
    (19, 16, "Exercises", ""),
    (20, 16, "Pussy-cat", ""),
    (21, 17, "Silver Sledges", "M. Yordansky"),
    (22, 17, "Exercise", ""),
    (23, 17, "Jolly Ganders", ""),
    (24, 17, "Study", "E. Gnesina"),
    // III. Sharps, flats and naturals
    (25, 18, "Which of Us Is Clever?", "An. Aleksandrov"),
    (26, 18, "Raven", "M. Rauchverger"),
    (27, 19, "Study", "E. Gnesina"),
    (28, 19, "Birds", "four hands"),
    (29, 19, "Story", "Yu. Abelyev"),
    (30, 20, "Study", "E. Gnesina"),
    (31, 20, "Exercise", ""),
    // IV. Smooth playing (legato)
    (32, 20, "Exercise", ""),
    (33, 21, "Little Shepherd", ""),
    (34, 21, "Latvian Folk Song", ""),
    (35, 21, "Exercise", ""),
    (36, 21, "Crane", "M. Krasyev"),
    (37, 22, "Birds' House", "M. Rauchverger"),
    (38, 22, "Latvian Folk Song", ""),
    (39, 22, "Autumn", ""),
    (40, 23, "Byelorussian Folk Song", ""),
    (41, 23, "Study", "E. Gnesina"),
    (42, 23, "Georgian Folk Song", ""),
    (43, 23, "Study", "E. Gnesina"),
    (44, 23, "Study", "E. Gnesina"),
    (45, 24, "Spring Song", ""),
    // V. Quavers
    (46, 24, "Exercise", ""),
    (47, 25, "The Wind Blew on the Sea", "M. Krasyev"),
    (48, 25, "Shade", "V. Kalinnikov"),
    (49, 25, "Autumn Song", "D. Vasilyev-Buglai"),
    (50, 26, "River Visla", "Polish folk song"),
    (51, 26, "Little Cossack", ""),
    (52, 27, "Hungarian Folk Song", ""),
    (53, 27, "The Young Girl Walked in the Pine Forest", "Russian folk song, four hands"),
    (54, 27, "Winter's Past", "N. Metlov"),
    (55, 28, "The Shepherd's Pipe", "children's song"),
    (56, 28, "Peter's Song", "D. Kabalevsky, four hands"),
    (57, 29, "A Russian Song", "I. Dunayevsky"),
    (58, 29, "Ukrainian Folk Song", ""),
    (59, 30, "Exercise", ""),
    (60, 30, "Step by Step", "children's song"),
    (61, 30, "Melody", "children's song"),
    (62, 30, "Study", "E. Gnesina"),
    (63, 31, "Ukrainian Folk Song", ""),
    (64, 31, "Latvian Folk Song", ""),
    (65, 31, "Song", ""),
    (66, 31, "Study", "G. Humbert"),
    (67, 32, "Melody Study", "P. Vasilyev"),
    (68, 32, "The Little Shepherd", "children's song"),
    (69, 32, "Exercise", ""),
    (70, 32, "On the Hill", "Russian folk song"),
    (71, 33, "The Woodcutter", "Russian folk song"),
    (72, 33, "Folk Song", "P. Tchaikovsky, four hands"),
    (73, 34, "Winter", "An. Aleksandrov"),
    (74, 34, "My Little Cow", "Russian folk song"),
    (75, 35, "Piece", "J. Weckerlin"),
    (76, 35, "Our Land", "D. Kabalevsky"),
    (77, 35, "Folk Song", ""),
    (78, 36, "Exercise", ""),
    (79, 36, "On the Lawn", "Russian folk song"),
    (80, 36, "Yugoslav Folk Song", "arr. A. Kondratyev"),
    (81, 37, "Children's Dance", ""),
    (82, 37, "My Field", "Russian folk song"),
    (83, 37, "Under a Green Apple Tree", "Russian folk song"),
    (84, 38, "Lullaby", "Azerbaijani folk song"),
    (85, 38, "Folk Song", "K. Sorokin"),
    (86, 39, "In My Little Garden", "Russian folk song"),
    (87, 39, "A Round Dance", "Russian folk song, arr. A. Nikolaev"),
    (88, 39, "Study", "N. Lyubarsky"),
    (89, 40, "The Amateur Angler", "M. Starokadomsky, four hands"),
];

pub const RS1B: &[PieceRef] = &[
    (90, 8, "The Shepherd Plays", "T. Salyutrinskaya"),
    (91, 8, "The Rain is Drizzling", "An. Aleksandrov"),
    (92, 9, "German Song", "L. Kershner"),
    (93, 9, "Lullaby", "I. Philippe"),
    (94, 10, "On Television: Let's Begin", "A. Ostrovsky, four hands"),
    (95, 10, "Study", "E. Gnesina"),
    (96, 11, "Study", "E. Gnesina"),
    (97, 11, "Exercise", ""),
    (98, 11, "Study", "E. Gnesina"),
    (99, 12, "The Joker", "Polish folk song, four hands"),
    (100, 13, "Study", "E. Gnesina"),
    (101, 13, "Echo", "Zh. Arman"),
    (102, 14, "A Song About a Peewit", "M. Yordansky, four hands"),
    (103, 15, "Kazakh Folk Song", "grace notes"),
    (104, 15, "The Sparrow", "A. Rubbakh"),
    (105, 15, "Exercise", ""),
    (106, 16, "Russian Song", "E. Aglintsova"),
    (107, 16, "Study", "E. Gnesina – double notes"),
    (108, 16, "Study", "E. Gnesina – double notes"),
    (109, 16, "Exercise", "chords"),
    (110, 17, "Winter", "M. Krutitsky"),
    (111, 17, "Study", "E. Gnesina"),
    (112, 18, "Chorus from 'Ivan Susanin'", "M. Glinka, four hands"),
    (113, 19, "Piece", "D. Levidova"),
    (114, 19, "Chinese Song", "M. Rauchverger"),
    (115, 19, "Willow", "Russian folk song"),
    (116, 20, "Study", "E. Gnesina"),
    (117, 20, "Exercise", ""),
    (118, 21, "At the Gate the Wind is Blowing", "M. Glinka, four hands"),
    (119, 22, "Rigaudon", "A. Goedike"),
    (120, 22, "Piece", "Zh. Arman"),
    (121, 23, "Homeland", "Russian folk song"),
    (122, 23, "Gavotte", "G. Telemann"),
    (123, 23, "Valse", "A. Rubbakh"),
    (124, 24, "Russian Folk Song", "A. Goedike"),
    (125, 24, "Folk Song", "arr. A. Nikolaev"),
    (126, 25, "Papageno's Aria", "W. A. Mozart, four hands"),
    (127, 26, "Study", "A. Nikolaev"),
    (128, 27, "On the Lawn", "Russian folk song, four hands"),
    (129, 27, "Piece", "V. Kurochkin"),
    (130, 28, "Study", "I. Berkovich"),
    (131, 28, "Study", "D. Levidova"),
    (132, 28, "Exercise", ""),
    (133, 29, "Echo", "A. Parusinov – triplets"),
    (134, 29, "Song", "E. Melartin"),
    (135, 30, "In the Garden", "S. Maikapar"),
    (136, 31, "Ukrainian Folk Song", "arr. S. Shevchenko"),
    (137, 31, "Song", "B. Bartók"),
    (138, 32, "Little Fisherman", "Latvian dance, four hands"),
    (139, 33, "Valse", "D. Kabalevsky"),
    (140, 34, "Cavalry Song of the Steppe", "L. Knipper, four hands"),
    (141, 36, "Study", "E. Gnesina"),
    (142, 37, "Study", "E. Gnesina – silent finger change"),
    (143, 38, "Adagio", "D. Steibelt"),
    (144, 39, "Minuet", "L. Mozart"),
    (145, 40, "Valse from 'Sleeping Beauty'", "P. Tchaikovsky, four hands"),
    (146, 41, "Bourrée", "J. Krieger"),
    (147, 42, "The Little Boy", "L. Kershner"),
    (148, 42, "In the Green Meadow", "A. Grechaninov"),
    (149, 44, "Prelude", "E. Tetsel"),
    (150, 44, "Exercise", ""),
    (151, 45, "The Little Shepherd", "S. Maikapar"),
    (152, 46, "Polka", "B. Tobis"),
    (153, 48, "Study", "A. Goedike"),
    (154, 49, "Dance", "A. Goedike"),
    (155, 50, "Song About a Stranger", "D. Shostakovich, four hands"),
    (156, 52, "German Dance", "F. Schubert, four hands"),
    (157, 54, "Musette", "L. Mozart"),
    (158, 54, "Exercise", ""),
    (159, 55, "Birds", "A. Karamanov"),
    (160, 56, "Cradle Song", "B. Fliess, four hands"),
    (161, 58, "Mazurka", "A. Grechaninov"),
    (162, 59, "March", "D. Shostakovich"),
    (163, 60, "Dance from 'Swan Lake'", "P. Tchaikovsky, four hands"),
];

pub fn piece_ref(number: u32) -> Option<(BookId, PieceRef)> {
    RS1A.iter().find(|p| p.0 == number).map(|p| (BookId::RussianSchool1a, *p)).or_else(|| RS1B.iter().find(|p| p.0 == number).map(|p| (BookId::RussianSchool1b, *p)))
}

/// Scale/chord/arpeggio tables at the back of each Russian School book.
pub const RS1A_SCALE_TABLE: usize = 43;
pub const RS1B_SCALE_TABLE: usize = 65;
