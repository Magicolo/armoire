use armoire::*;
use checkito::*;
use std::{error, result};

type Result = result::Result<(), Box<dyn error::Error>>;
const COUNT: usize = 1024;

#[test]
fn get_inserted_value_by_key() -> Result {
    char::generator().check(COUNT, |&value| {
        let mut armoire = Armoire::new();
        let key = armoire.insert(value);
        prove!(armoire.get(key) == Some(&value))
    })?;
    Ok(())
}

#[test]
fn get_mut_inserted_value_by_key() -> Result {
    isize::generator().check(COUNT, |&(mut value)| {
        let mut armoire = Armoire::new();
        let key = armoire.insert(value);
        prove!(armoire.get_mut(key) == Some(&mut value))
    })?;
    Ok(())
}

#[test]
fn iter_has_inserted_key_value() -> Result {
    u32::generator().check(COUNT, |&value| {
        let mut armoire = Armoire::new();
        let key = armoire.insert(value);
        let pairs = armoire.iter().collect::<Vec<_>>();
        prove!(pairs.len() == 1)?;
        prove!(pairs.get(0) == Some(&(key, &value)))
    })?;
    Ok(())
}

#[test]
fn iter_mut_has_inserted_key_value() -> Result {
    bool::generator().check(COUNT, |&(mut value)| {
        let mut armoire = Armoire::new();
        let key = armoire.insert(value);
        let pairs = armoire.iter_mut().collect::<Vec<_>>();
        prove!(pairs.len() == 1)?;
        prove!(pairs.get(0) == Some(&(key, &mut value)))
    })?;
    Ok(())
}

// #[test]
// fn scope_writes_twice() {
//     let mut armoire = Armoire::new();
//     let key = armoire.insert(1u8);
//     armoire.scope(|mut write, _, _| {
//         *write.get_mut(key)? += 1;
//         assert_eq!(*write.get_mut(key)?, 2);
//         Some(())
//     });
// }

pub enum Action {
    Insert(usize, bool),
    Remove(usize, bool),
    Clear,
    Drain,
    Resolve,
}

pub fn model() {}
