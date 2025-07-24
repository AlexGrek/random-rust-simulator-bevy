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

    /// Checks if the direction is orthogonal (North, East, South, West).
    ///
    /// # Returns
    /// `true` if the direction is orthogonal, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// use crate::Direction; // Assuming Direction is in the same crate
    /// assert_eq!(Direction::N.is_orthogonal(), true);
    /// assert_eq!(Direction::NE.is_orthogonal(), false);
    /// ```
    pub fn is_orthogonal(&self) -> bool {
        matches!(self, Direction::N | Direction::E | Direction::S | Direction::W)
    }

    /// Checks if the direction is diagonal (North-East, South-East, South-West, North-West).
    ///
    /// # Returns
    /// `true` if the direction is diagonal, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// use crate::Direction; // Assuming Direction is in the same crate
    /// assert_eq!(Direction::NE.is_diagonal(), true);
    /// assert_eq!(Direction::E.is_diagonal(), false);
    /// ```
    pub fn is_diagonal(&self) -> bool {
        matches!(self, Direction::NE | Direction::SE | Direction::SW | Direction::NW)
    }

       /// Returns the coordinates of the one or two neighbors reached by moving in this direction.
    ///
    /// For orthogonal directions, it returns the single direct neighbor duplicated twice
    /// in the array.
    /// For diagonal directions, it returns the two orthogonal neighbors that compose the diagonal
    /// movement.
    ///
    /// # Arguments
    /// * `x` - The current x-coordinate.
    /// * `y` - The current y-coordinate.
    ///
    /// # Returns
    /// An array `[(usize, usize); 2]` containing the neighbor coordinates.
    ///
    /// # Examples
    /// ```
    /// use crate::Direction; // Assuming Direction is in the same crate
    /// // Orthogonal
    /// assert_eq!(Direction::N.get_next_from(10, 10), [(10, 9), (10, 9)]);
    /// assert_eq!(Direction::E.get_next_from(10, 10), [(11, 10), (11, 10)]);
    ///
    /// // Diagonal
    /// assert_eq!(Direction::NE.get_next_from(10, 10), [(10, 9), (11, 10)]); // North and East components
    /// assert_eq!(Direction::SW.get_next_from(10, 10), [(10, 11), (9, 10)]); // South and West components
    /// ```
    pub fn get_next_from(&self, x: usize, y: usize) -> [(usize, usize); 2] {
        match self {
            Direction::N => {
                let next_y = y.saturating_sub(1);
                [(x, next_y), (x, next_y)]
            }
            Direction::S => {
                let next_y = y + 1;
                [(x, next_y), (x, next_y)]
            }
            Direction::E => {
                let next_x = x + 1;
                [(next_x, y), (next_x, y)]
            }
            Direction::W => {
                let next_x = x.saturating_sub(1);
                [(next_x, y), (next_x, y)]
            }
            Direction::NE => {
                let north = (x, y.saturating_sub(1));
                let east = (x + 1, y);
                [north, east]
            }
            Direction::SE => {
                let south = (x, y + 1);
                let east = (x + 1, y);
                [south, east]
            }
            Direction::SW => {
                let south = (x, y + 1);
                let west = (x.saturating_sub(1), y);
                [south, west]
            }
            Direction::NW => {
                let north = (x, y.saturating_sub(1));
                let west = (x.saturating_sub(1), y);
                [north, west]
            }
        }
    }

    /// Calculates the direct next coordinate based on the current direction.
    /// This returns a single point representing the destination of the movement.
    ///
    /// # Arguments
    /// * `x` - The current x-coordinate.
    /// * `y` - The current y-coordinate.
    ///
    /// # Returns
    /// A tuple `(usize, usize)` representing the next coordinates.
    ///
    /// # Examples
    /// ```
    /// use crate::Direction; // Assuming Direction is in the same crate
    /// assert_eq!(Direction::N.get_direct_next_point(10, 10), (10, 9));
    /// assert_eq!(Direction::NE.get_direct_next_point(10, 10), (11, 9));
    /// ```
    pub fn get_direct_next_point(&self, x: usize, y: usize) -> (usize, usize) {
        match self {
            Direction::N => (x, y.saturating_sub(1)),
            Direction::NE => (x + 1, y.saturating_sub(1)),
            Direction::E => (x + 1, y),
            Direction::SE => (x + 1, y + 1),
            Direction::S => (x, y + 1),
            Direction::SW => (x.saturating_sub(1), y + 1),
            Direction::W => (x.saturating_sub(1), y),
            Direction::NW => (x.saturating_sub(1), y.saturating_sub(1)),
        }
    }

    /// Returns the orthogonal components of a diagonal direction.
    /// For orthogonal directions, it returns the direction itself.
    ///
    /// # Returns
    /// A tuple `(Option<Direction>, Option<Direction>)` representing the orthogonal components.
    /// For orthogonal directions, the second element will be `None`.
    ///
    /// # Examples
    /// ```
    /// use crate::Direction; // Assuming Direction is in the same crate
    /// assert_eq!(Direction::N.orthogonal_components(), (Some(Direction::N), None));
    /// assert_eq!(Direction::NE.orthogonal_components(), (Some(Direction::N), Some(Direction::E)));
    /// ```
    pub fn orthogonal_components(&self) -> (Option<Direction>, Option<Direction>) {
        match self {
            Direction::N => (Some(Direction::N), None),
            Direction::E => (Some(Direction::E), None),
            Direction::S => (Some(Direction::S), None),
            Direction::W => (Some(Direction::W), None),
            Direction::NE => (Some(Direction::N), Some(Direction::E)),
            Direction::SE => (Some(Direction::S), Some(Direction::E)),
            Direction::SW => (Some(Direction::S), Some(Direction::W)),
            Direction::NW => (Some(Direction::N), Some(Direction::W)),
        }
    }
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
