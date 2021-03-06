use game_state;
use std;
use rand;
use time;
use std::collections::HashSet;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub struct UCTData{
    //represents data used by UCB1 to choose the best choice to explored
    //the win-tie field is for better statistics, not actually used
    pub wins : f64,
    pub num_plays : i32,
    pub win_tie : i32
}

impl UCTData{
    fn new(w : f64, n : i32) -> UCTData{
        UCTData{
            wins : w,
            num_plays : n,
            win_tie : 0
        }
    }

    fn win_percentage(&self) -> f64{
        (self.win_tie as f64 / self.num_plays as f64)
    }
}

pub struct TreePolicyResult{
    pub path : Vec<game_state::GameState>,
    pub expanded_node : game_state::GameState
}

impl TreePolicyResult{
    pub fn new(path : Vec<game_state::GameState>, 
        expanded_node : game_state::GameState) -> TreePolicyResult{
            TreePolicyResult{
                path : path,
                expanded_node : expanded_node
            }
    }
}

fn ucb1(win_value : f64, number_played : f64, total_played : f64) -> f64{
    //weighs exploration and expected output
    ((2f64 * total_played.ln()) / number_played).sqrt() + win_value / number_played
}

pub fn victory(end : game_state::End) -> bool{
    //simple helper function
    match end{
        game_state::End::Victory(_) => true,
        game_state::End::Tie => true,
        _ => false
    }
}

pub fn choose_random(possible_moves : &Vec<game_state::Move>) -> game_state::Move{
    let random_number = rand::random::<usize>() % possible_moves.len();
    let random_move = possible_moves[random_number].clone();
    return random_move;
}

pub fn run_simulation(state : game_state::GameState, player : game_state::Color) -> game_state::End{ 
    //from a given state, it will continue to choose random legitimate options until one player wins or ties
    let mut current_state = state;
    while !victory(current_state.win()){
        let current_player = current_state.player;
        let possible_moves = state.legal_moves(current_player);
        if possible_moves.len() < 1{
            break;
        }
        let random_move = choose_random(&possible_moves);
        current_state = current_state.place(&random_move);
    }

    current_state.win()
}

fn get_result_value(result : game_state::End, player : game_state::Color) -> f64{
    //returns the "reward" of each multi-armed bandit
    //a tie is better than a loss, but not as good as a win
    match result{
        game_state::End::Tie => 0.5f64,
        game_state::End::Victory(color) =>{
            if color == player{
                1f64
            }
            else{
                0f64
            }
        },
        _ => 0f64
    }
}

fn get_tie_or_win(result : game_state::End, player : game_state::Color) -> i32{
    //same function, but ties are also one. for the tie-win statistic
    match result{
        game_state::End::Tie => 1,
        game_state::End::Victory(color) =>{
            if color == player{
                1
            }
            else{
                0
            }
        },
        _ => 0
    }
}

fn state_previous_player(state : &game_state::GameState) -> game_state::Color{
    //helper function
    //the board member player represents the player who goes next
    match state.player{
        game_state::Color::White => game_state::Color::Black,
        game_state::Color::Black => game_state::Color::White,
        _ => game_state::Color::White
    }
}

pub fn tree_search(root : game_state::GameState) -> game_state::Move{

    //keeps track of visisted states so we know if current state is a leaf
    let mut visited_states : HashSet<game_state::GameState> = std::collections::HashSet::new();
    visited_states.insert(root);
    let mut statistics : HashMap<game_state::GameState, UCTData> = HashMap::new();
    statistics.insert(root, UCTData::new(0f64, 0));

    let current_time = time::precise_time_s();
    //temp
    while time::precise_time_s() - current_time < 3.5f64{
        let current_state = root;

        //selection
        let selected_state = tree_policy(&current_state, &visited_states, &statistics);

        //expand
        if !visited_states.contains(&selected_state.expanded_node){
            statistics.insert(selected_state.expanded_node, UCTData::new(0f64, 0));
            visited_states.insert(selected_state.expanded_node);
        }

        //simulate
        let result = run_simulation(selected_state.expanded_node, root.player);

        //backpropogate
        back_propogate(result, &mut statistics, &selected_state.path);
    }

    let possible_moves = root.legal_moves(root.player).into_iter().map(|x| (x, statistics.get(&root.place(&x)).unwrap())).collect::<Vec<_>>();
    let best_move = optimal_move_most_visisted(&possible_moves);
    let data = statistics.get(&root.place(&best_move)).unwrap();
    println!("Puny human, I have thought through {} variations of this pitiful game, and won or tied in {}% of them", data.num_plays, data.win_percentage() * 100f64);
    return best_move;
}

fn optimal_move_highest_win(possible_moves : &Vec<(game_state::Move, &UCTData)>) -> game_state::Move{
    //selects the highest winning node as optimal
    let mut highest_win = 0f64;
    let mut best_move = game_state::Move::white_new(0);
    for &(mv, data) in possible_moves{
        if data.wins > highest_win{
            highest_win = data.wins;
            best_move = mv;
        }
    }
    return best_move;
}

fn optimal_move_most_visisted(possible_moves : &Vec<(game_state::Move, &UCTData)>) -> game_state::Move{
    //selects the most visited node as optimal
    let mut most_played = 0;
    let mut best_move = game_state::Move::white_new(0);
    for &(mv, data) in possible_moves{
        if data.num_plays > most_played{
            most_played = data.num_plays;
            best_move = mv;
        }
    }
    return best_move;
}



pub fn tree_policy(
    current_state : &game_state::GameState,
    visisted_states : &HashSet<game_state::GameState>,
    stats : &HashMap<game_state::GameState, UCTData>
    ) -> TreePolicyResult{
    
    //represents the states we went through to get to the selected node
    //used for backpropogation without an actual tree structure
    let mut path : Vec<game_state::GameState> = Vec::new();

    let mut current_node = current_state.clone();

    loop{

        path.push(current_node);

        let possible_moves = current_node.legal_moves(current_node.player);

        if possible_moves.len() < 1 || victory(current_node.win()){
            //no legal moves or game ends
            return TreePolicyResult::new(path, current_node);
        }
        
        //has every possible move been explored?
        let fully_explored = possible_moves.iter().fold(true, 
            |acc, x| 
            acc && visisted_states.contains(&current_node.place(x))
        );

        //if not, exploration
        if !fully_explored {
            //for a node with number played of 0, ucb1 returns infinity
            //in other words unexplored child nodes are always explored at least once
            let not_explored = possible_moves.into_iter().filter(
                |x| !visisted_states.contains(&current_node.place(x))
                ).collect::<Vec<_>>();
            let random_choice = choose_random(&not_explored);
            let chosen_node = current_node.place(&random_choice);
            path.push(chosen_node);
            let result = TreePolicyResult::new(path, chosen_node);
            return result; 
        }

        //all child nodes have been simulated at least once, so use ucb1 to select best
        else{
            //sort 
            let mut best_move = possible_moves.last().unwrap();
            let mut best_uct = 0f64;
            let total_played = stats.get(&current_node).unwrap().num_plays;
            for possibility in possible_moves.iter(){
                
                //TODO: switch to pattern matching
                let data = stats.get(&current_node.place(&possibility)).unwrap();
                let uct = ucb1(data.wins, data.num_plays as f64, total_played as f64);
                if(uct > best_uct){
                    best_uct = uct;
                    best_move = possibility;
                }
            }
            let chosen_node = current_node.place(&best_move);
            current_node = chosen_node;
        }
    }
}


pub fn back_propogate(win_value : game_state::End, stats : &mut HashMap<game_state::GameState, UCTData>,
    path : &Vec<game_state::GameState>){
        for node in path.iter(){
            match stats.get_mut(node){
                Some(ref mut stat) =>{
                    stat.wins += get_result_value(win_value, state_previous_player(&node));
                    stat.num_plays += 1;
                    stat.win_tie += get_tie_or_win(win_value, state_previous_player(&node));
                }
                None => ()
            }
        }
}
