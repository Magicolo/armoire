use armoire::*;
use rayon::prelude::*;
use std::{
    mem::replace,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Player {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
    pub target: Option<Key>,
}

#[derive(Clone)]
pub struct Enemy {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
}

#[derive(Clone)]
pub enum Entity {
    Player(Player),
    Enemy(Enemy),
}

pub struct Time {
    pub total: Duration,
    pub delta: Duration,
}

pub struct Resources {
    pub time: Time,
}

pub struct Context<'a> {
    pub key: Key,
    pub resources: &'a Resources,
    pub entities: Read<'a, Entity>,
    pub defer: Defer<'a, Entity>,
}

impl From<Player> for Entity {
    fn from(value: Player) -> Self {
        Self::Player(value)
    }
}
impl From<Enemy> for Entity {
    fn from(value: Enemy) -> Self {
        Self::Enemy(value)
    }
}

impl Entity {
    pub fn step(&mut self, context: Context) {
        match self {
            Entity::Player(player) => player.step(context),
            Entity::Enemy(enemy) => enemy.step(context),
        }
    }
}

impl Player {
    pub fn step(
        &mut self,
        Context {
            resources: Resources { time },
            entities,
            defer,
            ..
        }: Context,
    ) {
        self.position[0] += self.velocity[0] * time.delta.as_secs_f64();
        self.position[1] += self.velocity[1] * time.delta.as_secs_f64();

        let target_pair = entities.iter().min_by_key(|(_, target)| match target {
            Entity::Enemy(enemy) => {
                Some((distance(self.position, enemy.position) * 1_000_000.0) as u64)
            }
            _ => None,
        });

        let target_key = match target_pair {
            Some((key, _)) => key,
            None => defer.insert(
                Enemy {
                    position: [0.0; 2],
                    velocity: [0.0; 2],
                }
                .into(),
            ),
        };

        self.target = Some(target_key);
    }
}

impl Enemy {
    pub fn step(&mut self, _context: Context) {}
}

impl Time {
    pub fn step(&mut self, now: Instant, then: &mut Instant) {
        let delta = now - replace(then, now);
        self.total += delta;
        self.delta = delta;
    }
}

fn main() {
    let mut armoire = Armoire::new();
    armoire.insert(Entity::Player(Player {
        position: [0.0; 2],
        velocity: [1.0; 2],
        target: None,
    }));
    armoire.insert(Entity::Enemy(Enemy {
        position: [0.0; 2],
        velocity: [1.0; 2],
    }));

    let mut resources = Resources {
        time: Time {
            total: Duration::ZERO,
            delta: Duration::ZERO,
        },
    };
    let mut then = Instant::now();
    while armoire
        .par_iter()
        .any(|(_, entity)| matches!(entity, Entity::Player(_)))
    {
        resources.time.step(Instant::now(), &mut then);
        armoire.scope(|mut write, read, defer| {
            write.par_iter_mut().for_each(|(key, entity)| {
                entity.step(Context {
                    key,
                    resources: &resources,
                    entities: read,
                    defer,
                })
            });
        });
    }
}

fn distance(left: [f64; 2], right: [f64; 2]) -> f64 {
    let x = left[0] + right[0];
    let y = left[1] + right[1];
    (x * x + y * y).sqrt()
}
