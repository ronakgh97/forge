use anyhow::Result;
use forge::api::agents::{Agent, AgentBuilder};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Color {
    Red,
    Blue,
    Green,
    Yellow,
    Wild,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Number(u8),
    Skip,
    Reverse,
    DrawTwo,
    Wild,
    WildDrawFour,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Card {
    color: Color,
    value: Value,
}

// Create Number card of given color, (19 per color, Total: 76 number cards)
fn create_76_number_card() -> Vec<Card> {
    let mut cards = Vec::new();
    let colors = [Color::Red, Color::Blue, Color::Green, Color::Yellow];

    for &color in &colors {
        // One 0 card
        cards.push(Card {
            color,
            value: Value::Number(0),
        });

        // Two of each 1-9 cards
        for number in 1..=9 {
            cards.push(Card {
                color,
                value: Value::Number(number),
            });
            cards.push(Card {
                color,
                value: Value::Number(number),
            });
        }
    }

    cards
}

// Create Action cards (8 per color, Total: 24 action cards)
fn create_24_action_cards() -> Vec<Card> {
    let mut cards = Vec::new();
    let colors = [Color::Red, Color::Blue, Color::Green, Color::Yellow];
    for &color in &colors {
        for _ in 0..2 {
            cards.push(Card {
                color,
                value: Value::Skip,
            });
            cards.push(Card {
                color,
                value: Value::Reverse,
            });
            cards.push(Card {
                color,
                value: Value::DrawTwo,
            });
        }
    }
    cards
}

// Create Wild cards (4 Wild, 4 Wild Draw Four, Total: 8 wild cards)
fn create_8_wild_cards() -> Vec<Card> {
    let mut cards = Vec::new();
    for _ in 0..4 {
        cards.push(Card {
            color: Color::Wild,
            value: Value::Wild,
        });
        cards.push(Card {
            color: Color::Wild,
            value: Value::WildDrawFour,
        });
    }
    cards
}

fn main() {
    // Pre-Checks
    {
        let mut deck = Vec::new();
        deck.extend(create_76_number_card());
        deck.extend(create_24_action_cards());
        deck.extend(create_8_wild_cards());

        assert_eq!(deck.len(), 108);
    }

    // Create Deck
    let mut deck = Vec::new();
    deck.extend(create_76_number_card());
    deck.extend(create_24_action_cards());
    deck.extend(create_8_wild_cards());

    // Shuffle Deck
    use rand::rng;
    use rand::seq::SliceRandom;
    let mut rng = rng();
    let mut shuffled_deck = deck.clone();
    shuffled_deck.shuffle(&mut rng);

    assert_ne!(deck, shuffled_deck);

    // Distribute Cards to Players
    let (distributed_cards, remaining) =
        distribute_cards_to_players(&mut shuffled_deck, 5).unwrap();

    dbg!(&distributed_cards);
    let total_distributed: usize = distributed_cards.iter().map(|hand| hand.len()).sum();
    assert_eq!(total_distributed + remaining.len(), 108);
}

fn distribute_cards_to_players(
    deck: &mut Vec<Card>,
    cards_per_player: usize,
) -> Result<([Vec<Card>; 4], Vec<Card>)> {
    // Hardcoded for 3 AI players, 1 Human player
    let mut players: Vec<Vec<Card>> = vec![Vec::new(); 4];
    for player_hand in players.iter_mut() {
        for _ in 0..cards_per_player {
            if let Some(card) = deck.pop() {
                player_hand.push(card);
            } else {
                return Err(anyhow::anyhow!("Not enough cards in the deck"));
            }
        }
    }
    Ok((
        [
            players[0].clone(),
            players[1].clone(),
            players[2].clone(),
            players[3].clone(),
        ],
        deck.clone(), // Remaining deck
    ))
}

#[allow(unused)]
fn draw_card(player_cards: &mut Vec<Card>) -> Option<Card> {
    player_cards.pop()
}

#[allow(unused)]
fn take_card_from_deck(deck: &mut Vec<Card>) -> Option<Card> {
    deck.pop()
}

#[allow(unused)]
fn create_ai_players() -> Vec<Agent> {
    const SYSTEM_PROMPT: &str = r#"You are playing the card game UNO. Make strategic decisions based on your hand and the game state to win the game."#;

    vec![
        AgentBuilder::new()
            .model("qwen/qwen3-8b")
            .url("http://localhost:1234/v1")
            .api_key("local")
            .system_prompt(SYSTEM_PROMPT)
            //.tool_registry()
            .build()
            .unwrap(),
        AgentBuilder::new()
            .model("qwen/qwen3-vl-8b")
            .url("http://localhost:1234/v1")
            .api_key("local")
            .system_prompt(SYSTEM_PROMPT)
            //.tool_registry()
            .build()
            .unwrap(),
        AgentBuilder::new()
            .model("zai-org/glm-4.6v-flash")
            .url("http://localhost:1234/v1")
            .api_key("local")
            .system_prompt(SYSTEM_PROMPT)
            //.tool_registry()
            .build()
            .unwrap(),
    ]
}
