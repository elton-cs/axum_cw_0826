pub struct Item {
    pub item_type: ItemType,
    pub player_id: usize,
    pub player_name: String,
    pub player_login_username: String,
    pub player_login_password: String,

    pub player_stat_gem: u32,
    pub player_stat_exp: u32,
    pub player_stat_lvl: u32,
    pub player_stat_frag_count: [u32; 26],
    pub player_stat_rune_count: [u32; 26],

    pub puzzle_level: u32,
    pub puzzle_level_next: usize,
    pub puzzle_correct_word: [char; 5],

    pub puzzle_attempt_num: u32,
    pub puzzle_attempt_next: usize,
    pub puzzle_attempt_word: [char; 5],
    pub puzzle_attempt_hint: [Hint; 5],

    pub puzzle_complete: bool,
    pub puzzle_reward_exp: u32,
    pub puzzle_reward_frag: char,

    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_frag_exp: u32,
    pub tile_rune_exp: u32,
    pub tile_frag_gem: u32,
    pub tile_rune_gem: u32,
    pub tile_frag_letter: char,
    pub tile_rune_letter: char,
    pub tile_frag_player_name: String,
    pub tile_rune_player_name: String,
}

pub enum ItemType {
    Player,
    PuzzleGame,
    PuzzleGameAttempt,
    Tile,
}

pub enum Hint {
    Missing,
    Present,
    Correct,
}
