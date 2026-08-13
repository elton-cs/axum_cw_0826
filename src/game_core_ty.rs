use std::collections::HashMap;

pub struct Game {
    pub user_map: HashMap<String, usize>,
    pub user: Vec<User>,
    pub tile: Vec<Vec<Tile>>,
}

pub enum GameCmd {
    CreatePlayer {
        name: String,
        pass: String,
    },
    GiftFreeGems {
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

pub fn update_game(game: &mut Game, game_cmd: GameCmd) -> Result<(), ()> {
    match game_cmd {
        GameCmd::CreatePlayer { name, pass } => {
            let user_key = format!("{name}{pass}");
            if game.user_map.contains_key(&user_key) {
                return Err(());
            }

            let user_idx = game.user.len();
            let new_user = User {
                name,
                pass,
                gem: 0,
                exp: 0,
                lvl: 0,
                frag_count: [0; 26],
                rune_count: [0; 26],
                curr_puzzle: None,
                prev_puzzle: Vec::new(),
            };

            game.user_map.insert(user_key, user_idx);
            game.user.push(new_user);
            Ok(())
        }
        GameCmd::GiftFreeGems { user_idx, gem_gift } => {
            if !user_idx < game.user.len() {
                return Err(());
            } else if game.user[user_idx].gem != 0 {
                return Err(());
            }

            game.user[user_idx].gem += gem_gift;
            Ok(())
        }
        GameCmd::BuyPuzzle { user_idx } => {
            if !user_idx < game.user.len() {
                return Err(());
            } else if game.user[user_idx].gem == 0 {
                return Err(());
            } else if game.user[user_idx].curr_puzzle.is_some() {
                return Err(());
            }

            let correct_word = "jesus".to_string();
            let puzzle = Some(Puzzle {
                reward_exp: 100,
                reward_frag: 'j',
                correct_word,
                attempt_word: Vec::new(),
            });

            let user = &mut game.user[user_idx];
            user.curr_puzzle = puzzle;

            Ok(())
        }
        GameCmd::GuessPuzzle {
            user_idx,
            guess_word,
        } => {
            if !user_idx < game.user.len() {
                return Err(());
            }

            let user = &mut game.user[user_idx];
            match &mut user.curr_puzzle {
                None => return Err(()),
                Some(puzzle) => {
                    // simple tmp check for correct guess
                    let hint = if puzzle.correct_word == guess_word {
                        [Hint::Missing; 5]
                    } else {
                        [Hint::Correct; 5]
                    };

                    let attempt = Attempt {
                        word: guess_word,
                        hint,
                    };

                    puzzle.attempt_word.push(attempt);
                    match puzzle.attempt_word.last() {
                        None => return Err(()),
                        Some(attempt) => {
                            if attempt.hint.iter().all(|h| matches!(h, Hint::Correct)) {
                                let frag_idx = puzzle.reward_frag as usize - 'a' as usize;
                                user.exp += puzzle.reward_exp;
                                user.frag_count[frag_idx] += 1;
                                user.prev_puzzle.push(puzzle.clone());
                                user.curr_puzzle = None;
                            }
                        }
                    }
                }
            }

            Ok(())
        }
        GameCmd::PlaceFrag {
            user_idx,
            user_name,
            x,
            y,
            frag,
        } => {
            if !user_idx < game.user.len() {
                return Err(());
            }
            let Some(row) = game.tile.get_mut(x) else {
                return Err(());
            };
            let Some(tile) = row.get_mut(y) else {
                return Err(());
            };

            if tile.frag_letter.is_some() {
                return Err(());
            }

            let user = &mut game.user[user_idx];
            let frag_idx = frag as usize - 'a' as usize;

            if user.frag_count[frag_idx] < 1 {
                return Err(());
            }

            user.gem += tile.frag_gem;
            user.exp += tile.frag_exp;
            tile.frag_exp = 0;
            tile.frag_gem = 0;
            tile.frag_letter = Some(frag);
            tile.frag_user_name = Some(user_name);

            Ok(())
        }
        GameCmd::PlaceRune {
            user_idx,
            user_name,
            x,
            y,
            rune,
        } => {
            if !user_idx < game.user.len() {
                return Err(());
            }
            let Some(row) = game.tile.get_mut(x) else {
                return Err(());
            };
            let Some(tile) = row.get_mut(y) else {
                return Err(());
            };

            if tile.rune_letter.is_some() {
                return Err(());
            }

            let user = &mut game.user[user_idx];
            let rune_idx = rune as usize - 'a' as usize;

            if user.rune_count[rune_idx] < 1 {
                return Err(());
            }

            match tile.frag_letter {
                Some(frag) => {
                    if !(frag == rune) {
                        return Err(());
                    }
                }
                None => return Err(()),
            }

            user.gem += tile.rune_gem;
            user.exp += tile.rune_exp;
            tile.rune_exp = 0;
            tile.rune_gem = 0;
            tile.rune_letter = Some(rune);
            tile.rune_user_name = Some(user_name);

            Ok(())
        }
    }
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
