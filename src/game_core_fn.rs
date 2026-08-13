use crate::game_core_ty::*;

pub fn update_game(game: &mut Game, game_cmd: GameCmd) -> Result<(), GameError> {
    match game_cmd {
        GameCmd::ServerPopulateTiles { x, y } => {
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
                        frag_exp: 1_000 + distance * 100,
                        rune_exp: 5_000 + distance * 500,
                        ..Tile::default()
                    });
                }
                game.tile.push(row);
            }

            Ok(())
        }
        GameCmd::CreatePlayer { name, pass } => {
            if game.user_map.contains_key(&name) {
                return Err(GameError::PlayerAlreadyExists);
            }

            let user_idx = game.user.len();
            let new_user = User {
                name: name.clone(),
                pass: pass.clone(),
                gem: 0,
                exp: 0,
                exp_next: 200,
                lvl: 0,
                frag_count: [0; 26],
                rune_count: [0; 26],
                curr_puzzle: None,
                prev_puzzle: Vec::new(),
            };

            game.user_map.insert(name, (user_idx, pass));
            game.user.push(new_user);
            Ok(())
        }
        GameCmd::ClaimFreeGems { user_idx, gem_gift } => {
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
            } else if game.user[user_idx].gem < PUZZLE_PRICE {
                return Err(GameError::InsufficientGems);
            } else if game.user[user_idx].curr_puzzle.is_some() {
                return Err(GameError::PuzzleAlreadyActive);
            } else if game.tile.is_empty() || game.tile.iter().any(|row| row.is_empty()) {
                return Err(GameError::TilesNotPopulated);
            }

            let word_idx = rand::random_range(0..PUZZLE_WORDS.len());
            let correct_word = PUZZLE_WORDS[word_idx].to_string();
            let reward_idx = rand::random_range(0..5);
            let reward_frag = correct_word.as_bytes()[reward_idx] as char;

            let reward_exp = 100 + game.user[user_idx].prev_puzzle.len() as u32 * 10;
            let puzzle = Some(Puzzle {
                reward_exp,
                reward_frag,
                correct_word,
                attempt_word: Vec::new(),
            });

            let user = &mut game.user[user_idx];
            user.curr_puzzle = puzzle;
            user.gem -= PUZZLE_PRICE;
            game.treasury_gem += TREASURY_FEE;
            game.protocol_gem += PROTOCOL_FEE;
            game.games_bought += 1;

            match PRIZE_OPTION.last() {
                None => panic!(),
                Some(highest_prize) => {
                    if &game.treasury_gem > highest_prize {
                        let rand_x_idx = rand::random_range(0..game.tile.len());
                        let rand_y_idx = rand::random_range(0..game.tile[rand_x_idx].len());
                        let rand_gem_idx = rand::random_range(0..PRIZE_OPTION.len());
                        game.is_frag_time = !game.is_frag_time;
                        if game.is_frag_time {
                            let prize = PRIZE_OPTION[rand_gem_idx];
                            game.tile[rand_x_idx][rand_y_idx].frag_gem += prize;
                            game.treasury_gem -= prize;
                        } else {
                            let prize = PRIZE_OPTION[rand_gem_idx];
                            game.tile[rand_x_idx][rand_y_idx].rune_gem += prize;
                            game.treasury_gem -= prize;
                        }
                    }
                }
            }

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
