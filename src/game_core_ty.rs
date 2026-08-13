use std::collections::HashMap;

pub const PUZZLE_PRICE: u32 = 10;
pub const TREASURY_FEE: u32 = 9;
pub const PROTOCOL_FEE: u32 = PUZZLE_PRICE - TREASURY_FEE;

pub const PRIZE_OPTION: [u32; 3] = [5, 10, 25];
pub const FRAG_GEM_REWARD_INTERVAL: u32 = 13;
pub const RUNE_GEM_REWARD_INTERVAL: u32 = 5;

pub const PUZZLE_WORDS: [&str; 20] = [
    "apple", "beach", "chair", "dance", "eagle", "flame", "grape", "house", "index", "jelly",
    "knife", "lemon", "mouse", "night", "ocean", "plant", "queen", "river", "stone", "tiger",
];

pub type Username = String;
pub type Password = String;
pub type UserIdx = usize;

pub struct Game {
    pub user_map: HashMap<Username, (UserIdx, Password)>,
    pub user: Vec<User>,
    pub tile: Vec<Vec<Tile>>,

    pub protocol_gem: u32,
    pub treasury_gem: u32,
    pub games_bought: u32,
    pub is_frag_time: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    PlayerAlreadyExists,
    PlayerNotFound {
        user_idx: usize,
    },
    PlayerAlreadyHasGems,
    InsufficientGems,
    PuzzleAlreadyActive,
    NoActivePuzzle,
    InvalidGuessLength {
        expected: usize,
        actual: usize,
    },
    InvalidLetter {
        letter: char,
    },
    PuzzleAttemptNotRecorded,
    TilesAlreadyPopulated,
    TilesNotPopulated,
    InvalidTileDimensions {
        x: usize,
        y: usize,
    },
    TileNotFound {
        x: usize,
        y: usize,
    },
    FragmentAlreadyPlaced,
    FragmentNotOwned {
        fragment: char,
    },
    InsufficientFragments {
        frag: char,
        required: u32,
        available: u32,
    },
    RuneAlreadyPlaced,
    RuneNotOwned {
        rune: char,
    },
    FragmentRequiredBeforeRune,
    RuneDoesNotMatchFragment {
        rune: char,
        fragment: char,
    },
}

pub enum GameCmd {
    ServerPopulateTiles {
        x: usize,
        y: usize,
    },
    CreatePlayer {
        name: String,
        pass: String,
    },
    ClaimFreeGems {
        user_idx: usize,
        gem_gift: u32,
    },
    BuyPuzzle {
        user_idx: usize,
    },
    GuessPuzzle {
        user_idx: usize,
        guess_word: String,
    },
    CraftSingleRune {
        user_idx: usize,
        frag: char,
    },
    CraftRandomRune {
        user_idx: usize,
        frags: [char; 5],
    },
    PlaceFrag {
        user_idx: usize,
        user_name: String,
        x: usize,
        y: usize,
        frag: char,
    },
    PlaceRune {
        user_idx: usize,
        user_name: String,
        x: usize,
        y: usize,
        rune: char,
    },
}

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub pass: String,

    pub gem: u32,
    pub exp: u32,
    pub exp_next: u32,
    pub lvl: u32,
    pub frag_count: [u32; 26],
    pub rune_count: [u32; 26],

    pub curr_puzzle: Option<Puzzle>,
    pub prev_puzzle: Vec<Puzzle>,
}

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub reward_exp: u32,
    pub reward_frag: char,
    pub correct_word: String,
    pub attempt_word: Vec<Attempt>,
}

#[derive(Debug, Clone)]
pub struct Attempt {
    pub word: String,
    pub hint: [Hint; 5],
}

#[derive(Debug, Clone, Copy)]
pub enum Hint {
    Missing,
    Present,
    Correct,
}

#[derive(Default)]
pub struct Tile {
    pub frag_exp: u32,
    pub rune_exp: u32,
    pub frag_gem: u32,
    pub rune_gem: u32,
    pub frag_letter: Option<char>,
    pub rune_letter: Option<char>,
    pub frag_user_name: Option<String>,
    pub rune_user_name: Option<String>,
}
