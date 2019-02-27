#[macro_use] extern crate serenity;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate regex;

use std::fs::File;
use std::io::Read;
use std::io::Result;
use std::collections::HashSet;

use regex::Regex;
use serenity::{
    model::{ channel::Channel, channel::Message, gateway::Ready, user::User },
    prelude::*,
};

fn get_file_string( path: &str ) -> Result< String > {
    let mut file = File::open( path )?;
    let mut result = String::new();

    file.read_to_string( &mut result )?;

    Ok( result )
}

#[derive(Serialize,Deserialize)]
struct Settings {
    token: String,
    shin_id: u64,
    public_roles: HashSet< String >,
    owner_role: String,
    staff_role: String,
    adult_role: String,
    mute_role: String
}

struct Handler {
    settings: Settings
}

impl EventHandler for Handler {

    fn message( &self, _: Context, message: Message ) {
        if self.is_mentioned( &message.mentions ) {
            let mut token_stack = self.get_token_stack( message.content.to_owned() );

            // Pop @Shinigami mention
            token_stack.pop();

            let command = token_stack.pop();
            if command.is_some() {
                let command = command.unwrap();
                match command.as_str() {
                    "add" => self.add_role( message, { token_stack.pop(); token_stack } ),
                    _ => self.say_unknown( message )
                }
            }
        }
    }

    fn ready( &self, _: Context, ready: Ready ) {
        println!( "{} is connected!", ready.user.name );
    }
}

impl Handler {

    fn get_token_stack( &self, message: String ) -> Vec< String > {
        let mut result = Vec::new();

        let regex = Regex::new( r#"[^\s"']+|"([^"]*)"|'([^']*)'"# ).expect( "fatal: could not parse regex" );

        for capture in regex.find_iter( &message ) {
            result.push( capture.as_str().to_owned() );
        }

        result.reverse();

        result
    }

    fn add_role( &self, message: Message, token_stack: Vec< String > ) {
        match message.channel() {
            Some( Channel::Guild( guild_channel ) ) => {
                let x = guild_channel.read();
                // TODO
            },
            _ => self.say( message, String::from( "This command cannot be used in this context." ) )
        }
    }

    fn say_unknown( &self, message: Message ) {
        self.say( message, String::from( "this isn't a valid command.") );
    }

    fn say( &self, message: Message, text: String ) {
         if let Err( why ) = message.channel_id.say( format!( "{}, {}", message.author.mention(), text ) ) {
            println!( "warning: Could not send message: {:?}", why );
         }
    }

    fn is_mentioned( &self, mentions: &Vec< User > ) -> bool {
        for user in mentions {
            if *user.id.as_u64() == self.settings.shin_id {
                return true
            }
        }

        false
    }

}

fn main() {
    let settings: Settings = toml::from_str( get_file_string( "settings.toml" ).expect( "fatal: could not open settings file" ).as_str() ).expect( "fatal: could not parse settings.toml" );

    let mut client = Client::new( &settings.token.to_owned(), Handler{ settings } ).expect( "fatal: could not create Discord client" );

    if let Err( why ) = client.start() {
        println!( "fatal: could not start Discord client: {:?}", why );
    }
}
