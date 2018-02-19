#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate rocket;
extern crate rocket_contrib;
// extern crate time;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod models;

use rand::Rng;

use rocket::request::{FlashMessage, Form, FromRequest, Outcome, Request};
use rocket::Outcome::Success;
use rocket::response::{Flash, NamedFile, Redirect};
use rocket::http::{Cookie, Cookies, RawStr};
use rocket_contrib::Template;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io;

// use time::get_time;

lazy_static! {
    pub static ref CHARACTERS: Characters = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Serialize, Clone)]
pub struct Character {
    name: String,
    strength: u8,
    dexterity: u8,
    hitpoints: u8,
}

#[derive(FromForm, Debug)]
pub struct CharacterForm {
    name: String,
}

type Characters = Arc<Mutex<HashMap<String, Character>>>;
struct GameState {
    players: Characters,
}

impl<'a, 'r> FromRequest<'a, 'r> for GameState {
    type Error = std::fmt::Error;
    fn from_request(_: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        Success(GameState {
            players: CHARACTERS.clone(),
        })
    }
}

#[derive(Serialize)]
struct GameViewData {
    flash: String,
    characters: HashMap<String, Character>,
}

#[get("/")]
fn index(cookies: Cookies) -> Redirect {
    match cookies.get("character_id") {
        Some(_) => Redirect::to("/game"),
        None => Redirect::to("/new"),
    }
}

#[get("/new")]
fn new() -> Template {
    Template::render("index", &())
}

#[post("/new", data = "<name>")]
fn post_new(
    mut cookies: Cookies,
    name: Form<CharacterForm>,
    state: GameState,
) -> Result<Redirect, String> {
    let character = name.get();
    let mut rng = rand::thread_rng();
    let new_character_id: String = rng.gen::<u64>().to_string();

    let ref mut players = *state.players.lock().unwrap();
    players.insert(
        new_character_id.clone(),
        Character {
            name: character.name.clone(),
            strength: rng.gen::<u8>(),
            dexterity: rng.gen::<u8>(),
            hitpoints: rng.gen::<u8>(),
        },
    );
    // let cookie = Cookie::build("character_id", new_character_id)
    // .path("/")
    // .secure(true)
    // .finish();
    // cookies.add(cookie);
    cookies.add(Cookie::new("character_id", new_character_id));
    Ok(Redirect::to("/game"))
    // Redirect::to("/game")
}

#[post("/attack/<id>")]
fn attack(cookies: Cookies, state: GameState, id: &RawStr) -> Flash<Redirect> {
    let ref mut players = *state.players.lock().unwrap();

    let attacker_id = cookies.get("character_id").unwrap().value();
    println!("got attacker id {:?}", attacker_id);
    let attacker = players.get(&attacker_id.to_string()).unwrap().clone();
    let defender = players.get(id.as_str()).unwrap().clone();

    let mut rng = rand::thread_rng();
    let damage: i16 = attacker.strength as i16 - defender.dexterity as i16 + rng.gen::<i8>() as i16;

    let message = if damage < 1 {
        format!("{} missed {}", attacker.name, defender.name)
    } else if defender.hitpoints as i16 - damage < 1 {
        players.remove(id.as_str());
        format!(
            "{} hits {}. {} is slain!",
            attacker.name, defender.name, defender.name
        )
    } else {
        let new_defender = Character {
            name: defender.name.clone(),
            strength: defender.strength,
            dexterity: defender.dexterity,
            hitpoints: defender.hitpoints - damage as u8,
        };
        players.insert(id.as_str().to_string(), new_defender);
        format!("{} hits {}", attacker.name, defender.name)
    };

    Flash::error(Redirect::to("/game"), message)
}

#[get("/game")]
fn game(state: GameState, flash: Option<FlashMessage>) -> Template {
    let players = state.players.clone();
    let characters = players.lock().unwrap();
    let flash: String = match flash {
        Some(f) => f.msg().into(),
        None => "".into(),
    };

    let gvd = GameViewData {
        flash: flash,
        characters: characters.clone(),
    };
    Template::render("game", &gvd)
}

#[get("/<path..>", rank = 1)]
fn static_files(path: PathBuf) -> io::Result<NamedFile> {
    NamedFile::open(Path::new("static/").join(path))
}

fn main() {
    let post = models::Post {
        id: "hello".to_string(),
    };
    println!("{:?}", post);
    rocket::ignite().
        mount("/", routes![index, new, post_new, game, attack, static_files]).
        // attach(Template::custom(|engines| {
        //     engines.handlebars.register_helper("")
        // }))
        attach(Template::fairing()).
        launch();
}
