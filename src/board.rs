use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn next(&self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Cell(pub Option<Player>);

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.as_ref() {
            Some(player) => player.fmt(f),
            None => " ".fmt(f),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Board(pub [[Cell; 3]; 3]);

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "┬───┬───┬───┬")?;
        for rowi in 0..3 {
            let [c0, c1, c2] = self.0[rowi];
            writeln!(f, "│ {} │ {} │ {} │", c0, c1, c2)?;
            if rowi + 1 < 3 {
                writeln!(f, "├───┼───┼───┼")?;
            }
        }
        writeln!(f, "├───┴───┴───┘")?;
        Ok(())
    }
}

impl std::ops::Index<(usize, usize)> for Board {
    type Output = Cell;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.0[index.0][index.1]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Board {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.0[index.0][index.1]
    }
}

impl Board {
    pub fn winner(&self) -> Option<Player> {
        for &coords in &[
            // rows
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            // cols
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            // diag
            [(0, 0), (1, 1), (2, 2)],
            [(2, 0), (1, 1), (0, 2)],
        ] {
            let [a, b, c] = [self[coords[0]].0, self[coords[1]].0, self[coords[2]].0];
            if a.is_some() && a == b && a == c {
                return a;
            }
        }

        None
    }

    pub fn try_move(&mut self, player: Player, row: usize, col: usize) -> Result<(), ()> {
        if row >= 3 || col >= 3 {
            return Err(());
        }

        let cell = &mut self.0[row][col].0;

        if cell.is_some() {
            return Err(());
        }

        *cell = Some(player);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Game {
    pub board: Board,
    pub current_player: Player,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: Default::default(),
            current_player: Player::X,
        }
    }

    pub fn try_move(&mut self, player: Player, row: usize, col: usize) -> Result<(), ()> {
        if self.current_player != player {
            return Err(());
        }

        if let Ok(_) = self.board.try_move(player, row, col) {
            self.current_player = player.next();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winner_works() {
        let mut b = Board::default();

        assert_eq!(b.winner(), None);

        b[(0, 0)] = Cell(Some(Player::X));
        b[(1, 1)] = Cell(Some(Player::X));
        b[(2, 2)] = Cell(Some(Player::X));

        assert_eq!(b.winner(), Some(Player::X));

        b[(1, 1)] = Cell(Some(Player::O));

        assert_eq!(b.winner(), None);
    }

    #[test]
    fn serialize_deserialize() {
        let mut b1 = Board::default();

        b1[(0, 0)] = Cell(Some(Player::X));
        b1[(1, 1)] = Cell(Some(Player::X));

        let mut data = Vec::new();

        serde_cbor::to_writer(&mut data, &b1).unwrap();

        let b2 = serde_cbor::from_reader(&*data).unwrap();

        assert_eq!(b1, b2);
    }
}
