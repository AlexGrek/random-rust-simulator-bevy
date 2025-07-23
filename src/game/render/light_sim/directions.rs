#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Direction {
    N  = 0,
    NE = 1,
    E  = 2,
    SE = 3,
    S  = 4,
    SW = 5,
    W  = 6,
    NW = 7,
}

impl Direction {
    pub const ALL: [Direction; 8] = [
        Direction::N,
        Direction::NE,
        Direction::E,
        Direction::SE,
        Direction::S,
        Direction::SW,
        Direction::W,
        Direction::NW,
    ];
}

impl From<Direction> for usize {
    fn from(dir: Direction) -> usize {
        dir as usize
    }
}

impl TryFrom<usize> for Direction {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Direction::N),
            1 => Ok(Direction::NE),
            2 => Ok(Direction::E),
            3 => Ok(Direction::SE),
            4 => Ok(Direction::S),
            5 => Ok(Direction::SW),
            6 => Ok(Direction::W),
            7 => Ok(Direction::NW),
            _ => Err(()),
        }
    }
}
