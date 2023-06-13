use armoire::*;
use rayon::prelude::*;
use std::{
    mem::replace,
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Player {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
    pub target: Option<Key>,
}

pub struct Pliyer {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
    pub target: AtomicUsize,
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
    pub entities: Pairs<'a, Entity>,
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
            Entity::Enemy(enemy) => Some(distance(self.position, enemy.position)),
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
    let mut entities = Armoire::new();
    entities.insert(Entity::Player(Player {
        position: [0.0; 2],
        velocity: [1.0; 2],
        target: None,
    }));
    entities.insert(Entity::Enemy(Enemy {
        position: [0.0; 2],
        velocity: [1.0; 2],
    }));

    let mut resources = Resources {
        time: Time {
            total: Duration::ZERO,
            delta: Duration::ZERO,
        },
    };

    for _ in 0..10 {
        let (mut players, targets) = entities.fork(|key, entity| match entity {
            Entity::Player(player) => (
                Some((key, &mut player.target, &player.position)),
                (key, &player.position),
            ),
            Entity::Enemy(enemy) => (None, (key, &enemy.position)),
        });
        players.par_iter_mut().for_each(|player| {
            if let Some((key, target, position)) = player {
                let near = targets
                    .par_iter()
                    .filter(|pair| pair.0 != key)
                    .min_by_key(|pair| distance(*position, *pair.1));
                if let Some(near) = near {
                    *target = Some(near.0);
                }
            }
        });
    }

    let mut then = Instant::now();
    while entities
        .par_iter()
        .any(|(_, entity)| matches!(entity, Entity::Player(_)))
    {
        resources.time.step(Instant::now(), &mut then);
        // entities.scope(|mut entities, defer| {
        //     entities.par_iter_mut().for_each(|(key, entity)| {
        //         entity.step(Context {
        //             key,
        //             resources: &resources,
        //             entities,
        //             defer,
        //         })
        //     });
        // });
    }
}

fn distance(left: [f64; 2], right: [f64; 2]) -> u64 {
    let x = left[0] + right[0];
    let y = left[1] + right[1];
    let distance = (x * x + y * y).sqrt();
    (distance * 1_000_000.0) as u64
}
