extern crate astar;
extern crate rand;

use self::rand::{Rng};

pub struct WorldMap {
    width: i32,
    height: i32,
    tiles: Vec<Tile>
}

impl WorldMap {
    pub fn generate<R: Rng>(rng: &mut R, width: i32, height: i32) -> (Self, Location) {
        assert!(width > 0);
        assert!(height > 0);

        let tiles: Vec<_> = ::std::iter::repeat(Terrain::Nothing)
            .take((width * height) as usize)
            .map(|terrain| Tile::new(terrain))
            .collect();

        let mut world = WorldMap { width: width, height: height, tiles: tiles };

        // Generate rooms.
        let mut rooms: Vec<Room> = Vec::new();
        for _ in 0..60 {
            let room_width = rng.gen_range::<i32>(3, 15);
            let room_height = rng.gen_range::<i32>(3, 15);
            let room_x = rng.gen_range::<i32>(0, width - room_width);
            let room_y = rng.gen_range::<i32>(0, height - room_height);
            let room = Room::new(room_x, room_y, room_width, room_height);
            let mut available = true;
            for chosen in rooms.iter() {
                if chosen.overlaps(&room) {
                    available = false;
                    break;
                }
            }
            if available {
                //println!("{}x{} @ {}x{}", room_width, room_height, room_x, room_y);
                rooms.push(room);
            } else {
                //println!("Couldn't fit it");
            }
        }

        // Draw rooms.
        for room in rooms.iter() {
            for wall in room.walls() {
                world.get_tile_mut(*wall).terrain = Terrain::Wall;
            }

            for floor in room.floors() {
                world.get_tile_mut(*floor).terrain = Terrain::Floor;
            }
        }

        // Draw paths between rooms.
        for _ in 0..1 {
            // Pick two random walls from two random rooms.
            let wall1 = rooms.iter().random(rng).walls().random(rng);
            let wall2 = rooms.iter().random(rng).walls().random(rng);

            // Dig out walls and find path.
            world.get_tile_mut(*wall1).terrain = Terrain::Nothing;
            world.get_tile_mut(*wall2).terrain = Terrain::Nothing;
            println!("Searching for path from {:?} to {:?}...", wall1, wall2);
            match astar::astar(ConnectRooms::new(&world, *wall1, *wall2)) {
                Some(path) => {
                    for loc in path.iter() {
                        world.get_tile_mut(*loc).terrain = Terrain::Debug;
                    }
                },
                None => { println!("Failed to find path"); }
            }
        }

        // Pick a random floor in a random room to start on.
        let starting_loc = *rooms.iter().random(rng).floors().random(rng);

        (world, starting_loc)
    }
    pub fn tiles(&self) -> TileIterator {
        TileIterator::new(&self.tiles, self.width)
    }
    pub fn get_tile(&self, loc: Location) -> &Tile {
        let index = (loc.y * self.width + loc.x) as usize;
        assert!(index < self.tiles.len());
        &self.tiles[index]
    }
    pub fn get_tile_mut(&mut self, loc: Location) -> &mut Tile {
        let index = (loc.y * self.width + loc.x) as usize;
        assert!(index < self.tiles.len());
        &mut self.tiles[index]
    }
    fn get_adjacent(&self, loc: Location) -> Vec<Location> {
        let mut adjacent = Vec::new();
        if loc.x > 0 { adjacent.push(Location::new(loc.x - 1, loc.y)); }
        if loc.y > 0 { adjacent.push(Location::new(loc.x, loc.y - 1)); }
        if loc.x < self.width - 1 { adjacent.push(Location::new(loc.x + 1, loc.y)); }
        if loc.y < self.height - 1 { adjacent.push(Location::new(loc.x, loc.y + 1)); }

        return adjacent;
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    pub x: i32,
    pub y: i32
}

impl Location {
    pub fn new(x: i32, y: i32) -> Self {
        Location {x: x, y: y}
    }
    pub fn manhattan(&self, other: &Location) -> i32 {
        let total_x = if self.x > other.x { self.x - other.x } else { other.x - self.x };
        let total_y = if self.y > other.y { self.y - other.y } else { other.y - self.y };

        total_x + total_y
    }
}

impl ::std::fmt::Debug for Location {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) ->
        Result<(), ::std::fmt::Error> {
        f.write_fmt(format_args!("({}, {})", self.x, self.y))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Terrain {
    Debug,
    Nothing,
    Floor,
    Wall
}

#[derive(Copy, Clone, Debug)]
pub struct Entity {
    id: u64
}

pub struct Tile {
    pub terrain: Terrain,
    pub entities: Vec<Entity>
}

impl Tile {
    pub fn new(terrain: Terrain) -> Self {
        Tile {terrain: terrain, entities: Vec::new()}
    }
}

pub struct TileIterator<'a> {
    tiles: &'a [Tile],
    width: i32,
    curr: usize
}

impl<'a> TileIterator<'a> {
    pub fn new(tiles: &'a [Tile], width: i32) -> Self {
        TileIterator {tiles: tiles, width: width, curr: 0}
    }
}

impl<'a> Iterator for TileIterator<'a> {
    type Item = (&'a Tile, Location);

    fn next(&mut self) -> Option<(&'a Tile, Location)> {
        if self.curr < self.tiles.len() {
            let this = self.curr as i32;
            self.curr += 1;
            Some((
                &self.tiles[this as usize],
                Location {
                    x: this % self.width,
                    y: this / self.width
                }))
        } else { None }
    }
}

// GENERATION STUFF.
// A feature in the world, consisting of some arrangement of terrain.
// Features consist of relative coordinates; they can be placed at any
// arbitrary location.
struct Feature {
    components: Vec<(Location, Terrain)>
}

impl Feature {
    pub fn new(components: Vec<(Location, Terrain)>) -> Self {
        Feature { components: components }
    }
    pub fn width(&self) -> i32 {
        if self.components.len() == 0 {
            return 0;
        }

        self.components.iter().map(|c| c.0.x).max().unwrap() -
            self.components.iter().map(|c| c.0.x).min().unwrap() + 1
    }
    pub fn height(&self) -> i32 {
        if self.components.len() == 0 {
            return 0;
        }

        self.components.iter().map(|c| c.0.y).max().unwrap() -
            self.components.iter().map(|c| c.0.y).min().unwrap() + 1
    }
    pub fn iter(&self) -> ::std::slice::Iter<(Location, Terrain)> {
        self.components.iter()
    }
}

enum HorizontalAlignment {
    Left,
    Center,
    Right
}

enum VerticalAlignment {
    Top,
    Center,
    Bottom
}

// Build features! Take the raw feature shape and translate it
// according to the given alignment and absolute location.
struct FeatureBuilder {
    components: Vec<(Location, Terrain)>,
    location: Location,
    horiz_align: HorizontalAlignment,
    vert_align: VerticalAlignment
}

impl FeatureBuilder {
    pub fn new(components: Vec<(Location, Terrain)>) -> Self {
        assert!(components.len() > 0);
        FeatureBuilder {
            components: components,
            location: Location::new(0, 0),
            horiz_align: HorizontalAlignment::Center,
            vert_align: VerticalAlignment::Center
        }
    }
    pub fn location(mut self, loc: Location) -> Self {
        self.location = loc;
        self
    }
    pub fn horiz_align(mut self, align: HorizontalAlignment) -> Self {
        self.horiz_align = align;
        self
    }
    pub fn vert_align(mut self, align: VerticalAlignment) -> Self {
        self.vert_align = align;
        self
    }
    pub fn build(&self) -> Feature {
        let horiz = match self.horiz_align {
            HorizontalAlignment::Left => {
                self.location.x - Self::calc_min_x(&self.components)
            },
            HorizontalAlignment::Center => {
                self.location.x - (Self::calc_min_x(&self.components) +
                    (Self::calc_max_x(&self.components) -
                    Self::calc_min_x(&self.components) + 1) / 2)
            },
            HorizontalAlignment::Right => {
                self.location.x - Self::calc_max_x(&self.components)
            }
        };
        let vert = match self.vert_align {
            VerticalAlignment::Top => {
                self.location.y - Self::calc_min_y(&self.components)
            },
            VerticalAlignment::Center => {
                self.location.y - (Self::calc_min_y(&self.components) +
                    (Self::calc_max_y(&self.components) -
                    Self::calc_min_y(&self.components) + 1) / 2)
            },
            VerticalAlignment::Bottom => {
                self.location.y - Self::calc_max_y(&self.components)
            }
        };
        println!("adjustment: ({}, {})", horiz, vert);

        let comps = self.components.iter()
            .map(|c| {
                (Location::new(c.0.x + horiz, c.0.y + vert), c.1)
            })
            .collect();

        Feature {components: comps}
    }

    fn calc_min_x(components: &[(Location, Terrain)]) -> i32 {
        components.iter().map(|c| c.0.x).min().unwrap()
    }
    fn calc_max_x(components: &[(Location, Terrain)]) -> i32 {
        components.iter().map(|c| c.0.x).max().unwrap()
    }
    fn calc_min_y(components: &[(Location, Terrain)]) -> i32 {
        components.iter().map(|c| c.0.y).min().unwrap()
    }
    fn calc_max_y(components: &[(Location, Terrain)]) -> i32 {
        components.iter().map(|c| c.0.y).max().unwrap()
    }
}

// A room in the world.
#[derive(Debug)]
struct Room {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    locs: Vec<Location>
}

impl Room {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        assert!(x >= 0 && y >= 0 && width > 0 && height > 0);
        let mut locs: Vec<Location> = Vec::with_capacity((width * height) as usize);
        for i in x..x+width {
            for j in y..y+height {
                locs.push(Location::new(i, j));
            }
        }

        Room {x: x, y: y, width: width, height: height, locs: locs}
    }
    pub fn overlaps(&self, other: &Room) -> bool {
        let (xmin1, xmax1, xmin2, xmax2) = (self.x, self.x + self.width,
            other.x, other.x + other.width);
        let (ymin1, ymax1, ymin2, ymax2) = (self.y, self.y + self.height,
            other.y, other.y + other.height);
        (xmax1 >= xmin2) && (xmax2 >= xmin1) && (ymax1 >= ymin2) && (ymax2 >= ymin1)
    }
    pub fn walls<'a>(&'a self) -> Box<Iterator<Item=&'a Location> + 'a> {
        Box::new(self.locs.iter().filter(move |loc| loc.x == self.x || loc.y == self.y ||
            loc.x == self.x + self.width - 1 || loc.y == self.y + self.height - 1))
    }
    pub fn floors<'a>(&'a self) -> Box<Iterator<Item=&'a Location> + 'a> {
        Box::new(self.locs.iter().filter(move |loc| loc.x > self.x && loc.y > self.y &&
            loc.x < self.x + self.width - 1 && loc.y < self.y + self.height - 1))
    }
}

// Trait to extend iterators to provide a random function.
trait IterRandomExt<T> {
    fn random<R: Rng>(&mut self, rng: &mut R) -> T;
}

impl<I> IterRandomExt<I::Item> for I where I: Iterator, I::Item: Clone {
    fn random<R: Rng>(&mut self, rng: &mut R) -> I::Item {
        let elements: Vec<_> = self.collect();
        assert!(elements.len() > 0);
        let random = rng.gen_range::<usize>(0, elements.len() - 1);
        elements[random].clone()
    }
}

// Iterates through neighbors; used for A* algorithm.
struct NeighborIterator {
    adjacent: Vec<Location>,
    current: usize
}

impl NeighborIterator {
    pub fn new(world: &WorldMap, loc: Location) -> Self {
        let adjacent = world.get_adjacent(loc).iter()
            .map(|x| *x)
            .filter(|loc| world.get_tile(*loc).terrain == Terrain::Nothing)
            .collect();

        NeighborIterator { adjacent: adjacent, current: 0 }
    }
}

impl Iterator for NeighborIterator {
    type Item = (Location, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.adjacent.len() {
            self.current += 1;
            Some((self.adjacent[self.current - 1], 1))
        } else {
            None
        }
    }
}

// Search problem for connecting rooms with A* algorithm.
struct ConnectRooms<'a> {
    world: &'a WorldMap,
    start: Location,
    end: Location
}

impl<'a> ConnectRooms<'a> {
    pub fn new(world: &'a WorldMap, start: Location, end: Location) -> Self {
        ConnectRooms { world: world, start: start, end: end }
    }
}

impl<'a> astar::SearchProblem<Location, i32, NeighborIterator> for ConnectRooms<'a> {
    fn start(&self) -> Location {
        self.start
    }
    fn is_end(&self, loc: &Location) -> bool {
        *loc == self.end
    }
    fn heuristic(&self, loc: &Location) -> i32 {
        loc.manhattan(&self.end)
    }
    fn neighbors(&self, at: &Location) -> NeighborIterator {
        NeighborIterator::new(&self.world, *at)
    }
}

#[test]
fn test_feature_size() {
    let feature = Feature::new(vec![
        (Location::new(0, 0), Terrain::Wall),
        (Location::new(1, 0), Terrain::Wall),
        (Location::new(1, 1), Terrain::Wall)]);
    assert_eq!(feature.width(), 2);
    assert_eq!(feature.height(), 2);
}

#[test]
fn test_build_feature() {
    // Set up some feature shape:
    //
    // ....
    // .##.
    // .##.
    // .#..
    // ....
    let comps = vec![
        (Location::new(1, 1), Terrain::Wall),
        (Location::new(2, 1), Terrain::Wall),
        (Location::new(1, 2), Terrain::Wall),
        (Location::new(2, 2), Terrain::Wall),
        (Location::new(1, 3), Terrain::Wall)
    ];

    // Place top left at (2,3).
    //
    // .....
    // .....
    // .....
    // ..##.
    // ..##.
    // ..#..
    // .....
    assert_eq!(FeatureBuilder::new(comps.clone())
        .vert_align(VerticalAlignment::Top)
        .horiz_align(HorizontalAlignment::Left)
        .location(Location::new(2, 3))
        .build().components,
        vec![
            (Location::new(2, 3), Terrain::Wall),
            (Location::new(3, 3), Terrain::Wall),
            (Location::new(2, 4), Terrain::Wall),
            (Location::new(3, 4), Terrain::Wall),
            (Location::new(2, 5), Terrain::Wall)
        ]);

    // Place bottom right at (5,2).
    //
    // ....##.
    // ....##.
    // ....#..
    // .......
    assert_eq!(FeatureBuilder::new(comps)
        .vert_align(VerticalAlignment::Bottom)
        .horiz_align(HorizontalAlignment::Right)
        .location(Location::new(5, 2))
        .build().components,
        vec![
            (Location::new(4, 0), Terrain::Wall),
            (Location::new(5, 0), Terrain::Wall),
            (Location::new(4, 1), Terrain::Wall),
            (Location::new(5, 1), Terrain::Wall),
            (Location::new(4, 2), Terrain::Wall)
        ]);

    // Set up a square feature shape.
    //
    // ###
    // ###
    // ###
    let square = vec![
        (Location::new(0, 0), Terrain::Wall),
        (Location::new(1, 0), Terrain::Wall),
        (Location::new(2, 0), Terrain::Wall),
        (Location::new(0, 1), Terrain::Wall),
        (Location::new(1, 1), Terrain::Wall),
        (Location::new(2, 1), Terrain::Wall),
        (Location::new(0, 2), Terrain::Wall),
        (Location::new(1, 2), Terrain::Wall),
        (Location::new(2, 2), Terrain::Wall),
    ];

    // Place center at (4, 1).
    //
    // ...###.
    // ...###.
    // ...###.
    // .......
    assert_eq!(FeatureBuilder::new(square)
        .vert_align(VerticalAlignment::Center)
        .horiz_align(HorizontalAlignment::Center)
        .location(Location::new(4, 1))
        .build().components,
        vec![
            (Location::new(3, 0), Terrain::Wall),
            (Location::new(4, 0), Terrain::Wall),
            (Location::new(5, 0), Terrain::Wall),
            (Location::new(3, 1), Terrain::Wall),
            (Location::new(4, 1), Terrain::Wall),
            (Location::new(5, 1), Terrain::Wall),
            (Location::new(3, 2), Terrain::Wall),
            (Location::new(4, 2), Terrain::Wall),
            (Location::new(5, 2), Terrain::Wall)
        ]);
}