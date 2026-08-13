use std::collections::HashMap;

pub const GAME_PUZZLE_PRICE: u32 = 10;

const PUZZLE_WORDS: [&str; 20] = [
    "apple", "beach", "chair", "dance", "eagle", "flame", "grape", "house", "index", "jelly",
    "knife", "lemon", "mouse", "night", "ocean", "plant", "queen", "river", "stone", "tiger",
];

pub struct Game {
    pub user_map: HashMap<String, usize>,
    pub user: Vec<User>,
    pub tile: Vec<Vec<Tile>>,
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
    PopulateTiles {
        x: usize,
        y: usize,
    },
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
    CraftSingleRune {
        user_idx: usize,
        frag: char,
    },
    CraftRandomRune {
        user_idx: usize,
        frags: [char; 5],
    },
    PlaceRune {
        user_idx: usize,
        user_name: String,
        x: usize,
        y: usize,
        rune: char,
    },
}

fn letter_idx(letter: char) -> Result<usize, GameError> {
    if letter.is_ascii_lowercase() {
        Ok((letter as u8 - b'a') as usize)
    } else {
        Err(GameError::InvalidLetter { letter })
    }
}

fn handle_lvl_up(exp: &u32, next_exp: &mut u32, lvl: &mut u32) {
    if exp >= next_exp {
        *next_exp += (*next_exp as f32 * 1.1) as u32;
        *lvl += 1;
    }
}

pub fn update_game(game: &mut Game, game_cmd: GameCmd) -> Result<(), GameError> {
    match game_cmd {
        GameCmd::PopulateTiles { x, y } => {
            if !game.tile.is_empty() {
                return Err(GameError::TilesAlreadyPopulated);
            }
            if x < 9 || y < 9 || x % 2 == 0 || y % 2 == 0 {
                return Err(GameError::InvalidTileDimensions { x, y });
            }

            let center_x = x / 2;
            let center_y = y / 2;
            for tile_x in 0..x {
                let mut row = Vec::with_capacity(y);
                for tile_y in 0..y {
                    let distance = tile_x.abs_diff(center_x) + tile_y.abs_diff(center_y);
                    let distance = distance as u32;
                    row.push(Tile {
                        frag_exp: 1_000u32.saturating_add(distance.saturating_mul(100)),
                        rune_exp: 5_000u32.saturating_add(distance.saturating_mul(500)),
                        ..Tile::default()
                    });
                }
                game.tile.push(row);
            }

            Ok(())
        }
        GameCmd::CreatePlayer { name, pass } => {
            let user_key = format!("{name}{pass}");
            if game.user_map.contains_key(&user_key) {
                return Err(GameError::PlayerAlreadyExists);
            }

            let user_idx = game.user.len();
            let new_user = User {
                name,
                pass,
                gem: 0,
                exp: 0,
                exp_next: 200,
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
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            } else if game.user[user_idx].gem != 0 {
                return Err(GameError::PlayerAlreadyHasGems);
            }

            game.user[user_idx].gem += gem_gift;
            Ok(())
        }
        GameCmd::BuyPuzzle { user_idx } => {
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            } else if game.user[user_idx].gem < GAME_PUZZLE_PRICE {
                return Err(GameError::InsufficientGems);
            } else if game.user[user_idx].curr_puzzle.is_some() {
                return Err(GameError::PuzzleAlreadyActive);
            }

            let word_idx = rand::random_range(0..PUZZLE_WORDS.len());
            let correct_word = PUZZLE_WORDS[word_idx].to_string();
            let reward_idx = rand::random_range(0..5);
            let reward_frag = correct_word.as_bytes()[reward_idx] as char;

            let reward_exp = 100u32
                .saturating_add((game.user[user_idx].prev_puzzle.len() as u32).saturating_mul(10));
            let puzzle = Some(Puzzle {
                reward_exp,
                reward_frag,
                correct_word,
                attempt_word: Vec::new(),
            });

            let user = &mut game.user[user_idx];
            user.gem -= GAME_PUZZLE_PRICE;
            user.curr_puzzle = puzzle;

            Ok(())
        }
        GameCmd::GuessPuzzle {
            user_idx,
            guess_word,
        } => {
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            }

            let user = &mut game.user[user_idx];
            match &mut user.curr_puzzle {
                None => return Err(GameError::NoActivePuzzle),
                Some(puzzle) => {
                    let guess_letters: Vec<char> = guess_word.chars().collect();
                    if guess_letters.len() != 5 {
                        return Err(GameError::InvalidGuessLength {
                            expected: 5,
                            actual: guess_letters.len(),
                        });
                    }

                    let correct_letters: Vec<char> = puzzle.correct_word.chars().collect();
                    let mut hint = [Hint::Missing; 5];
                    let mut matched = [false; 5];

                    for idx in 0..5 {
                        if guess_letters[idx] == correct_letters[idx] {
                            hint[idx] = Hint::Correct;
                            matched[idx] = true;
                        }
                    }

                    for guess_idx in 0..5 {
                        if matches!(hint[guess_idx], Hint::Correct) {
                            continue;
                        }

                        for correct_idx in 0..5 {
                            if !matched[correct_idx]
                                && guess_letters[guess_idx] == correct_letters[correct_idx]
                            {
                                hint[guess_idx] = Hint::Present;
                                matched[correct_idx] = true;
                                break;
                            }
                        }
                    }

                    let attempt = Attempt {
                        word: guess_word,
                        hint,
                    };

                    puzzle.attempt_word.push(attempt);
                    match puzzle.attempt_word.last() {
                        None => return Err(GameError::PuzzleAttemptNotRecorded),
                        Some(attempt) => {
                            if attempt.hint.iter().all(|h| matches!(h, Hint::Correct)) {
                                let frag_idx = puzzle.reward_frag as usize - 'a' as usize;
                                user.exp += puzzle.reward_exp;
                                handle_lvl_up(&user.exp, &mut user.exp_next, &mut user.lvl);

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
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            }
            let Some(row) = game.tile.get_mut(x) else {
                return Err(GameError::TileNotFound { x, y });
            };
            let Some(tile) = row.get_mut(y) else {
                return Err(GameError::TileNotFound { x, y });
            };

            if tile.frag_letter.is_some() {
                return Err(GameError::FragmentAlreadyPlaced);
            }

            let frag_idx = letter_idx(frag)?;
            let user = &mut game.user[user_idx];

            if user.frag_count[frag_idx] < 1 {
                return Err(GameError::FragmentNotOwned { fragment: frag });
            }

            user.frag_count[frag_idx] -= 1;
            user.gem += tile.frag_gem;
            user.exp += tile.frag_exp;
            handle_lvl_up(&user.exp, &mut user.exp_next, &mut user.lvl);

            tile.frag_exp = 0;
            tile.frag_gem = 0;
            tile.frag_letter = Some(frag);
            tile.frag_user_name = Some(user_name);

            Ok(())
        }
        GameCmd::CraftSingleRune { user_idx, frag } => {
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            }

            let frag_idx = letter_idx(frag)?;
            let user = &mut game.user[user_idx];

            if user.frag_count[frag_idx] < 3 {
                return Err(GameError::InsufficientFragments {
                    frag,
                    required: 3,
                    available: user.frag_count[frag_idx],
                });
            }

            user.frag_count[frag_idx] -= 3;
            user.rune_count[frag_idx] += 1;
            user.exp += 500;
            handle_lvl_up(&user.exp, &mut user.exp_next, &mut user.lvl);

            Ok(())
        }
        GameCmd::CraftRandomRune { user_idx, frags } => {
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            }

            let mut required_frags = [0_u32; 26];
            for frag in frags {
                let frag_idx = letter_idx(frag)?;
                required_frags[frag_idx] += 1;
            }

            let user = &mut game.user[user_idx];
            for (frag_idx, required) in required_frags.iter().copied().enumerate() {
                if user.frag_count[frag_idx] < required {
                    return Err(GameError::InsufficientFragments {
                        frag: char::from(b'a' + frag_idx as u8),
                        required,
                        available: user.frag_count[frag_idx],
                    });
                }
            }

            for (frag_idx, required) in required_frags.iter().copied().enumerate() {
                user.frag_count[frag_idx] -= required;
            }
            let rune_idx = rand::random_range(0..26);
            user.rune_count[rune_idx] += 1;
            user.exp += 250;
            handle_lvl_up(&user.exp, &mut user.exp_next, &mut user.lvl);

            Ok(())
        }
        GameCmd::PlaceRune {
            user_idx,
            user_name,
            x,
            y,
            rune,
        } => {
            if user_idx >= game.user.len() {
                return Err(GameError::PlayerNotFound { user_idx });
            }
            let Some(row) = game.tile.get_mut(x) else {
                return Err(GameError::TileNotFound { x, y });
            };
            let Some(tile) = row.get_mut(y) else {
                return Err(GameError::TileNotFound { x, y });
            };

            if tile.rune_letter.is_some() {
                return Err(GameError::RuneAlreadyPlaced);
            }

            let rune_idx = letter_idx(rune)?;
            let user = &mut game.user[user_idx];

            if user.rune_count[rune_idx] < 1 {
                return Err(GameError::RuneNotOwned { rune });
            }

            match tile.frag_letter {
                Some(fragment) => {
                    if fragment != rune {
                        return Err(GameError::RuneDoesNotMatchFragment { rune, fragment });
                    }
                }
                None => return Err(GameError::FragmentRequiredBeforeRune),
            }

            user.rune_count[rune_idx] -= 1;
            user.gem += tile.rune_gem;
            user.exp += tile.rune_exp;
            handle_lvl_up(&user.exp, &mut user.exp_next, &mut user.lvl);
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
