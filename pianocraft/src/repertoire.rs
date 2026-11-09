//! Built-in repertoire: graded public-domain / Creative Commons scores from the Mutopia Project
//! (https://www.mutopiaproject.org), converted to MIDI with LilyPond. Credits and licences are in
//! `repertoire/LICENSES.md`; each piece shows its licence in the app.

pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    pub composer: &'static str,
    /// Approximate grade (ABRSM-style, 1 = first pieces after a primer, 8 = advanced).
    pub grade: u8,
    pub license: &'static str,
    pub typesetter: &'static str,
    pub midi: &'static [u8],
}

pub const ALL: &[Entry] = &[
    Entry { id: "am-anh131", title: "Air in F, BWV Anh. 131", composer: "Anna Magdalena Notebook", grade: 1, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Anonymous", midi: include_bytes!("../repertoire/am-anh131.mid") },
    Entry { id: "am-anh120", title: "Minuet in A minor, BWV Anh. 120", composer: "Anna Magdalena Notebook", grade: 1, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh120.mid") },
    Entry { id: "am-anh113", title: "Minuet in F, BWV Anh. 113", composer: "Anna Magdalena Notebook", grade: 1, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/am-anh113.mid") },
    Entry { id: "am-anh118", title: "Minuet in B♭, BWV Anh. 118", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh118.mid") },
    Entry { id: "am-anh121", title: "Minuet in C minor, BWV Anh. 121", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh121.mid") },
    Entry { id: "am-anh116", title: "Minuet in G, BWV Anh. 116", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/am-anh116.mid") },
    Entry { id: "am-anh128", title: "Polonaise in D minor, BWV Anh. 128", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh128.mid") },
    Entry { id: "am-anh117b", title: "Polonaise in F, BWV Anh. 117b", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh117b.mid") },
    Entry { id: "am-anh119", title: "Polonaise in G minor, BWV Anh. 119", composer: "Anna Magdalena Notebook", grade: 2, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh119.mid") },
    Entry { id: "am-anh115", title: "Minuet in G minor, BWV Anh. 115", composer: "Christian Petzold (Anna Magdalena Notebook)", grade: 2, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/am-anh115.mid") },
    Entry { id: "handel-menuet-dm", title: "Minuet in D minor", composer: "G. F. Handel – Aylesford Pieces", grade: 2, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/handel-menuet-dm.mid") },
    Entry { id: "schumann-68-4", title: "Chorale, Op. 68 No. 4", composer: "R. Schumann – Album for the Young", grade: 2, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-4.mid") },
    Entry { id: "schumann-68-3", title: "Humming Song, Op. 68 No. 3", composer: "R. Schumann – Album for the Young", grade: 2, license: "Creative Commons Attribution-ShareAlike 3.0", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-3.mid") },
    Entry { id: "schumann-68-5", title: "Little Piece, Op. 68 No. 5", composer: "R. Schumann – Album for the Young", grade: 2, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-5.mid") },
    Entry { id: "schumann-68-1", title: "Melody, Op. 68 No. 1", composer: "R. Schumann – Album for the Young", grade: 2, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-1.mid") },
    Entry { id: "schumann-68-2", title: "Soldiers' March, Op. 68 No. 2", composer: "R. Schumann – Album for the Young", grade: 2, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-2.mid") },
    Entry { id: "am-anh127", title: "March in E♭, BWV Anh. 127", composer: "Anna Magdalena Notebook", grade: 3, license: "Public Domain", typesetter: "Steven McDougall", midi: include_bytes!("../repertoire/am-anh127.mid") },
    Entry { id: "handel-air", title: "Air in B♭", composer: "G. F. Handel – Aylesford Pieces", grade: 3, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/handel-air.mid") },
    Entry { id: "handel-gavotte", title: "Gavotte in G", composer: "G. F. Handel – Aylesford Pieces", grade: 3, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/handel-gavotte.mid") },
    Entry { id: "handel-menuet-d", title: "Minuet in D", composer: "G. F. Handel – Aylesford Pieces", grade: 3, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/handel-menuet-d.mid") },
    Entry { id: "burg-100-1", title: "La Candeur (Candour), Op. 100 No. 1", composer: "J. F. Burgmüller – 25 Easy Studies", grade: 3, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/burg-100-1.mid") },
    Entry { id: "burg-100-3", title: "La Pastorale, Op. 100 No. 3", composer: "J. F. Burgmüller – 25 Easy Studies", grade: 3, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/burg-100-3.mid") },
    Entry { id: "clementi-36-1-i", title: "Sonatina in C, Op. 36 No. 1 – I. Allegro", composer: "M. Clementi", grade: 3, license: "Public Domain", typesetter: "Brian D. Rude", midi: include_bytes!("../repertoire/clementi-36-1-i.mid") },
    Entry { id: "clementi-36-1-ii", title: "Sonatina in C, Op. 36 No. 1 – II. Andante", composer: "M. Clementi", grade: 3, license: "Public Domain", typesetter: "Brian D. Rude", midi: include_bytes!("../repertoire/clementi-36-1-ii.mid") },
    Entry { id: "clementi-36-1-iii", title: "Sonatina in C, Op. 36 No. 1 – III. Vivace", composer: "M. Clementi", grade: 3, license: "Public Domain", typesetter: "Brian D. Rude", midi: include_bytes!("../repertoire/clementi-36-1-iii.mid") },
    Entry { id: "tchaik-39-1", title: "Morning Prayer, Op. 39 No. 1", composer: "P. Tchaikovsky – Children's Album", grade: 3, license: "Public Domain", typesetter: "Rem Zolotykh", midi: include_bytes!("../repertoire/tchaik-39-1.mid") },
    Entry { id: "tchaik-39-16", title: "Old French Song, Op. 39 No. 16", composer: "P. Tchaikovsky – Children's Album", grade: 3, license: "Public Domain", typesetter: "David McNamara", midi: include_bytes!("../repertoire/tchaik-39-16.mid") },
    Entry { id: "schumann-68-6", title: "Poor Orphan, Op. 68 No. 6", composer: "R. Schumann – Album for the Young", grade: 3, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-6.mid") },
    Entry { id: "czerny-821-1", title: "Eight-bar Study No. 1, Op. 821", composer: "C. Czerny", grade: 4, license: "Public Domain", typesetter: "Manuel Castejon Limas", midi: include_bytes!("../repertoire/czerny-821-1.mid") },
    Entry { id: "czerny-821-2", title: "Eight-bar Study No. 2, Op. 821", composer: "C. Czerny", grade: 4, license: "Public Domain", typesetter: "Manuel Castejon Limas", midi: include_bytes!("../repertoire/czerny-821-2.mid") },
    Entry { id: "czerny-821-3", title: "Eight-bar Study No. 3, Op. 821", composer: "C. Czerny", grade: 4, license: "Public Domain", typesetter: "Manuel Castejon Limas", midi: include_bytes!("../repertoire/czerny-821-3.mid") },
    Entry { id: "chopin-28-7", title: "Prelude in A, Op. 28 No. 7", composer: "F. Chopin", grade: 4, license: "Public Domain", typesetter: "", midi: include_bytes!("../repertoire/chopin-28-7.mid") },
    Entry { id: "burg-100-5", title: "Innocence, Op. 100 No. 5", composer: "J. F. Burgmüller – 25 Easy Studies", grade: 4, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/burg-100-5.mid") },
    Entry { id: "burg-100-2", title: "L'Arabesque, Op. 100 No. 2", composer: "J. F. Burgmüller – 25 Easy Studies", grade: 4, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/burg-100-2.mid") },
    Entry { id: "burg-100-8", title: "La Gracieuse, Op. 100 No. 8", composer: "J. F. Burgmüller – 25 Easy Studies", grade: 4, license: "Public Domain", typesetter: "Bas Wassink", midi: include_bytes!("../repertoire/burg-100-8.mid") },
    Entry { id: "tchaik-39-5", title: "March of the Wooden Soldiers, Op. 39 No. 5", composer: "P. Tchaikovsky – Children's Album", grade: 4, license: "Public Domain", typesetter: "Anonymous", midi: include_bytes!("../repertoire/tchaik-39-5.mid") },
    Entry { id: "schumann-68-7", title: "Hunting Song, Op. 68 No. 7", composer: "R. Schumann – Album for the Young", grade: 4, license: "Creative Commons Attribution-ShareAlike 3.0", typesetter: "Philippe Hézaine", midi: include_bytes!("../repertoire/schumann-68-7.mid") },
    Entry { id: "grieg-12-7", title: "Album Leaf, Op. 12 No. 7", composer: "E. Grieg – Lyric Pieces", grade: 5, license: "Creative Commons Attribution-ShareAlike 4.0", typesetter: "Knute Snortum", midi: include_bytes!("../repertoire/grieg-12-7.mid") },
    Entry { id: "satie-gnoss-1", title: "Gnossienne No. 1", composer: "E. Satie", grade: 5, license: "Creative Commons Attribution-ShareAlike 4.0", typesetter: "Knute Snortum", midi: include_bytes!("../repertoire/satie-gnoss-1.mid") },
    Entry { id: "satie-gymno-1", title: "Gymnopédie No. 1", composer: "E. Satie", grade: 5, license: "Public Domain", typesetter: "Evin Robertson", midi: include_bytes!("../repertoire/satie-gymno-1.mid") },
    Entry { id: "chopin-28-6", title: "Prelude in B minor, Op. 28 No. 6", composer: "F. Chopin", grade: 5, license: "Public Domain", typesetter: "", midi: include_bytes!("../repertoire/chopin-28-6.mid") },
    Entry { id: "chopin-28-20", title: "Prelude in C minor, Op. 28 No. 20", composer: "F. Chopin", grade: 5, license: "Public Domain", typesetter: "Magnus Lewis-Smith", midi: include_bytes!("../repertoire/chopin-28-20.mid") },
    Entry { id: "chopin-28-4", title: "Prelude in E minor, Op. 28 No. 4", composer: "F. Chopin", grade: 5, license: "Public Domain", typesetter: "Magnus Lewis-Smith", midi: include_bytes!("../repertoire/chopin-28-4.mid") },
    Entry { id: "bach-846", title: "Prelude in C, BWV 846", composer: "J. S. Bach – Well-Tempered Clavier I", grade: 5, license: "Public Domain", typesetter: "Tobias Erbsland", midi: include_bytes!("../repertoire/bach-846.mid") },
    Entry { id: "fur-elise", title: "Für Elise, WoO 59", composer: "L. van Beethoven", grade: 5, license: "Public Domain", typesetter: "Stelios Samelis", midi: include_bytes!("../repertoire/fur-elise.mid") },
    Entry { id: "chopin-69-2", title: "Waltz in B minor, Op. 69 No. 2", composer: "F. Chopin", grade: 6, license: "Public Domain", typesetter: "Roland Goretzki", midi: include_bytes!("../repertoire/chopin-69-2.mid") },
    Entry { id: "bach-inv-1", title: "Invention No. 1 in C, BWV 772", composer: "J. S. Bach", grade: 6, license: "Creative Commons Attribution-ShareAlike 3.0", typesetter: "jeff covey", midi: include_bytes!("../repertoire/bach-inv-1.mid") },
    Entry { id: "bach-inv-13", title: "Invention No. 13 in A minor, BWV 784", composer: "J. S. Bach", grade: 6, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/bach-inv-13.mid") },
    Entry { id: "bach-inv-4", title: "Invention No. 4 in D minor, BWV 775", composer: "J. S. Bach", grade: 6, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/bach-inv-4.mid") },
    Entry { id: "bach-inv-8", title: "Invention No. 8 in F, BWV 779", composer: "J. S. Bach", grade: 6, license: "Public Domain", typesetter: "Allen Garvin", midi: include_bytes!("../repertoire/bach-inv-8.mid") },
    Entry { id: "moonlight-1", title: "\"Moonlight\" Sonata, Op. 27 No. 2 – I. Adagio sostenuto", composer: "L. van Beethoven", grade: 6, license: "Creative Commons Attribution-ShareAlike 2.5", typesetter: "Stewart Holmes", midi: include_bytes!("../repertoire/moonlight-1.mid") },
    Entry { id: "schumann-traumerei", title: "Träumerei, Op. 15 No. 7", composer: "R. Schumann – Scenes from Childhood", grade: 6, license: "Public Domain", typesetter: "Ying-Chun Liu", midi: include_bytes!("../repertoire/schumann-traumerei.mid") },
    Entry { id: "joplin-entertainer", title: "The Entertainer", composer: "S. Joplin", grade: 6, license: "Public Domain", typesetter: "Chris Sawer", midi: include_bytes!("../repertoire/joplin-entertainer.mid") },
    Entry { id: "debussy-arabesque-1", title: "Arabesque No. 1", composer: "C. Debussy", grade: 7, license: "Public Domain", typesetter: "Keith OHara", midi: include_bytes!("../repertoire/debussy-arabesque-1.mid") },
    Entry { id: "clair-de-lune", title: "Clair de lune (Suite bergamasque)", composer: "C. Debussy", grade: 8, license: "Public Domain", typesetter: "Keith OHara", midi: include_bytes!("../repertoire/clair-de-lune.mid") },
];

/// The repertoire as practisable pieces.
pub fn pieces() -> Vec<crate::model::Piece> {
    ALL.iter()
        .filter_map(|e| {
            let mut p = crate::import::import_midi(e.midi, e.title).ok()?;
            p.id = format!("rep-{}", e.id);
            p.title = e.title.to_string();
            p.composer = e.composer.to_string();
            let who = if e.typesetter.is_empty() { String::new() } else { format!(", typeset by {}", e.typesetter) };
            p.notes_text = format!("Grade {} repertoire · {} · Mutopia Project{who}.", e.grade, e.license);
            p.origin = crate::model::Origin::BuiltIn;
            Some(p)
        })
        .collect()
}

pub fn grade_of(id: &str) -> Option<u8> {
    let id = id.strip_prefix("rep-")?;
    ALL.iter().find(|e| e.id == id).map(|e| e.grade)
}
