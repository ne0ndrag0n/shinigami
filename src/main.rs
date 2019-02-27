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
    model::{
        channel::Channel,
        channel::Message,
        gateway::Ready,
        user::User,
        id::{ UserId, GuildId, RoleId },
        guild::{ Member, Role }
    },
    prelude::*,
    utils::parse_username
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

enum ManageMode { Add, Remove }

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
                    "add" => self.manage_role( message, token_stack, ManageMode::Add ),
                    "remove" => self.manage_role( message, token_stack, ManageMode::Remove ),
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

    fn manage_role( &self, message: Message, mut token_stack: Vec< String >, mode: ManageMode ) {
        match message.guild() {
            Some( guild_rw_lock ) => {
                let target_role_id = match token_stack.pop() {
                    Some( value ) => {
                        let value = value.replace( "\"", "" );
                        if self.settings.public_roles.contains( &value.to_lowercase() ) {
                            match guild_rw_lock.read().role_by_name( &value ) {
                                Some( role_ref ) => role_ref.id,
                                None => { return self.say( message, String::from( "a fatal error has occurred (assertion failed: role could not be retrieved from guild)." ) ) }
                            }
                        } else {
                            return self.say( message, String::from( "this role is not publically assignable." ) )
                        }
                    },
                    None => { return self.say( message, String::from( "please provide a valid Role to assign." ) ) }
                };

                // Determine if we use message.author or a different target
                let mut target_member = match token_stack.pop() {
                    Some( value ) => /* match value.as_str() {
                        "to" => match token_stack.pop() {
                            Some( user_reference ) => match parse_username( &user_reference ) {
                                Some( user_numeric ) => match self.get_member_from_guild( &guild_rw_lock.read().id, &UserId( user_numeric ) ) {
                                    Some( member ) => member,
                                    None => { return self.say( message, String::from( "a fatal error has occurred (assertion failed: message author not retrievable from guild)." ) ) }
                                },
                                None => { return self.say( message, String::from( "a fatal error has occurred (assertion failed: target id could not be parsed to user id)." ) ) }
                            },
                            None => { return self.say( message, String::from( "syntax error. Usage: add \"role\" [ to @user ]" ) ) }
                        },
                        _ => { return self.say( message, String::from( "syntax error. Usage: add \"role\" [ to @user ]" ) ) }
                    }*/
                    { return self.say( message, String::from( "syntax error. Usage: add <role>" ) ) },
                    // Find message.author in guild_id
                    None => match self.get_member_from_guild( &guild_rw_lock.read().id, &message.author.id ) {
                        Some( guild_member ) => guild_member,
                        None => { return self.say( message, String::from( "a fatal error has occurred (assertion failed: message author not retrievable from guild)." ) ) }
                    }
                };

                // TODO: Verification that target_member has permission to apply target_role_id
                // for this MVP the target_member will always be message.author
                match mode {
                    ManageMode::Add => match target_member.add_role( target_role_id ) {
                        Ok( _ ) => self.say( message, String::from( "role added!" ) ),
                        Err( _ ) => self.say( message, String::from( "unable to add role!" ) )
                    },
                    ManageMode::Remove => match target_member.remove_role( target_role_id ) {
                        Ok( _ ) => self.say( message, String::from( "role removed!" ) ),
                        Err( _ ) => self.say( message, String::from( "unable to remove role!" ) )
                    }
                }
            },
            None => self.say( message, String::from( "this command is not valid in this context." ) )
        }
    }

    fn get_member_from_guild( &self, guild_id: &GuildId, user_id: &UserId ) -> Option< Member > {
        match guild_id.member( user_id ) {
            Ok( member ) => Some( member ),
            Err( _ ) => None
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
