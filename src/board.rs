use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Player {
    X,
    O,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
struct Cell(Option<Player>);

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(player) => player.fmt(f),
            None => " ".fmt(f),
        }
    }
}

#[derive(Debug, Default)]
struct Board {
    cells: [[Cell; 3]; 3],
}

impl Board {
    fn set(&mut self, p: Player, row: usize, col: usize) -> Result<(), ()> {
        if row >= self.cells.len() || col >= self.cells[0].len() {
            return Err(());
        }

        let mut x = self.cells[row][col];

        if x.0.is_some() {
            return Err(());
        }

        x = Cell(Some(p));

        Ok(())
    }

    fn has_winner(&self) -> Option<Player> {
        None
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "┬───┬───┬───┬")?;

        //        for row in self.cells.iter() {
        //            //row.iter().try_for_each(|cell| { write!(f, "| {} ", cell) })?;
        //            write!(f, "{}", row.iter().join(" | "))?;
        //            write!(f, " |\n")?;
        //        }

        for rowi in 0..3 {
            let row = &self.cells[rowi];
            writeln!(f, "│ {} │ {} │ {} │", row[0], row[1], row[2])?;
            if rowi + 1 < 3 {
                writeln!(f, "├───┼───┼───┼")?;
            }
        }

        writeln!(f, "├───┴───┴───┘")
    }
}

#[cfg(test)]
mod test {
    use crate::board::{Board, Cell, Player};

    #[test]
    fn display_cell() {
        assert_eq!("X", format!("{}", Cell(Some(Player::X))));
        assert_eq!("O", format!("{}", Cell(Some(Player::O))));
        assert_eq!(" ", format!("{}", Cell(None)));
    }

    #[test]
    fn display_board() {
        assert_eq!("X", format!("{}", Board::default()));
    }

    #[test]
    fn set_empty_cell() {
        assert!(Board::default().set(Player::X, 0, 0).is_ok());
    }

    #[test]
    fn set_nonempty_cell() {
        let mut board = Board::default();
        let _ = board.set(Player::X, 0, 0);
        assert!(board.set(Player::X, 0, 0).is_ok());
    }

    #[test]
    fn set_invalid_cell() {
        assert!(Board::default().set(Player::X, 3, 0).is_err());
    }

    #[test]
    fn default_board_does_not_have_winner() {
        assert!(Board::default().has_winner().is_none());
    }
}
