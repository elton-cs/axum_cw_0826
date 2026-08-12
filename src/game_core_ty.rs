pub struct Game {
    pub user: Vec<User>,
    pub tile: Vec<Vec<Tile>>,
}

pub struct User {
    pub name: String,
    pub pass: String,

    pub gem: u32,
    pub exp: u32,
    pub lvl: u32,
    pub frag_count: [u32; 26],
    pub rune_count: [u32; 26],

    pub curr_puzzle: Option<Puzzle>,
    pub prev_puzzle: Vec<Puzzle>,
}

pub struct Puzzle {
    pub reward_exp: u32,
    pub reward_frag: char,
    pub correct_word: String,
    pub attempt_word: Vec<Attempt>,
}

pub struct Attempt {
    pub word: String,
    pub hint: [Hint; 5],
}

pub enum Hint {
    Missing,
    Present,
    Correct,
}

pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub frag_exp: u32,
    pub rune_exp: u32,
    pub frag_gem: u32,
    pub rune_gem: u32,
    pub frag_letter: Option<char>,
    pub rune_letter: Option<char>,
    pub frag_user_name: Option<String>,
    pub rune_user_name: Option<String>,
}
