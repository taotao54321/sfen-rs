use crate::*;

const SFEN_STARTPOS: &str = "sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1";

/// sfen をパースして (局面、指し手リスト) を返す。
/// 合法性チェックは一切行わない。
pub fn decode(sfen: impl AsRef<str>) -> Result<(Position, Vec<Move>)> {
    let mut tokens = sfen.as_ref().split_ascii_whitespace();

    let pos = tokens_to_pos(&mut tokens)?;
    let mvs = tokens_to_moves(&mut tokens)?;

    Ok((pos, mvs))
}

fn tokens_to_pos<'a, I>(tokens: &mut I) -> Result<Position>
where
    I: Iterator<Item = &'a str>,
{
    let mut next = || {
        tokens
            .next()
            .ok_or_else(|| Error::decode_error("position: incomplete"))
    };

    let magic = next()?;
    match magic {
        "startpos" => tokens_to_pos(&mut SFEN_STARTPOS.split_ascii_whitespace()),
        "sfen" => {
            let s_board = next()?;
            let s_side = next()?;
            let s_hands = next()?;
            let s_ply = next()?;

            let board = decode_board(s_board)?;
            let side = decode_side(s_side)?;
            let (hand_sente, hand_gote) = decode_hands(s_hands)?;
            let ply = decode_ply(s_ply)?;

            Ok(Position::new(side, board, hand_sente, hand_gote, ply))
        }
        _ => Err(Error::decode_error(format!(
            "position: invalid magic: {}",
            magic
        ))),
    }
}

fn decode_board(s_board: impl AsRef<str>) -> Result<Board> {
    let s_board = s_board.as_ref();
    let it = s_board.split('/');

    let mut cells = [BoardCell::Empty; 81];
    for (y, s_row) in it.enumerate() {
        let row = decode_board_row(s_row)?;
        let idx = 9 * y;
        cells[idx..idx + 9].copy_from_slice(&row);
    }

    Ok(Board(cells))
}

fn decode_board_row(s_row: impl AsRef<str>) -> Result<[BoardCell; 9]> {
    #[derive(Debug)]
    struct State {
        row: [BoardCell; 9],
        len: usize,
        promo: bool,
    }
    impl State {
        fn new() -> Self {
            Self {
                row: [BoardCell::Empty; 9],
                len: 0,
                promo: false,
            }
        }
        fn eat(&mut self, c: char) -> Result<()> {
            match c {
                '+' => {
                    self.ensure_len_ok(1)?;
                    self.ensure_not_promo()?;
                    self.promo = true;
                }
                '1'..='9' => {
                    self.ensure_not_promo()?;
                    let n = c.to_digit(10).unwrap() as usize;
                    self.ensure_len_ok(n)?;
                    self.len += n;
                }
                _ => {
                    let (side, mut pt) = char_to_side_pt(c).ok_or_else(|| {
                        Error::decode_error(format!("board row: invalid char: {}", c))
                    })?;
                    self.ensure_len_ok(1)?;
                    if self.promo {
                        pt = pt.to_promoted().ok_or_else(|| {
                            Error::decode_error(format!("board row: not promotable piece: {}", c))
                        })?;
                        self.promo = false;
                    }
                    self.row[self.len] = BoardCell::Piece(side, pt);
                    self.len += 1;
                }
            }
            Ok(())
        }
        fn ensure_len_ok(&self, len_add: usize) -> Result<()> {
            if self.len + len_add > 9 {
                return Err(Error::decode_error("board row: overflow"));
            }
            Ok(())
        }
        fn ensure_not_promo(&self) -> Result<()> {
            if self.promo {
                return Err(Error::decode_error("board row: invalid '+'"));
            }
            Ok(())
        }
    }

    let mut state = State::new();
    for c in s_row.as_ref().chars() {
        state.eat(c)?;
    }

    Ok(state.row)
}

fn decode_side(s_side: impl AsRef<str>) -> Result<Side> {
    match s_side.as_ref() {
        "b" => Ok(Side::Sente),
        "w" => Ok(Side::Gote),
        s => Err(Error::decode_error(format!("side: invalid string: {}", s))),
    }
}

fn decode_hands(s_hands: impl AsRef<str>) -> Result<(Hand, Hand)> {
    let s_hands = s_hands.as_ref();
    if s_hands == "-" {
        return Ok((Hand::empty(), Hand::empty()));
    }

    #[derive(Debug)]
    struct State {
        counts: [[u8; 7]; 2],
        cur: u8,
    }
    impl State {
        fn new() -> Self {
            Self {
                counts: [[0; 7]; 2],
                cur: 0,
            }
        }
        fn eat(&mut self, c: char) -> Result<()> {
            match c {
                '0'..='9' => {
                    self.cur = self
                        .cur
                        .checked_mul(10)
                        .ok_or_else(|| Error::decode_error("hands: overflow"))?;
                    self.cur = self
                        .cur
                        .checked_add(c.to_digit(10).expect("internal error") as u8)
                        .ok_or_else(|| Error::decode_error("hands: overflow"))?;
                }
                _ => {
                    let (side, pt) = char_to_side_pt(c).ok_or_else(|| {
                        Error::decode_error(format!("hands: invalid char: {}", c))
                    })?;
                    if !pt.is_hand() {
                        return Err(Error::decode_error(format!("hands: not hand piece: {}", c)));
                    }
                    if self.cur == 0 {
                        self.cur = 1;
                    }
                    self.counts[side as usize][pt as usize] += self.cur;
                    self.cur = 0;
                }
            }
            Ok(())
        }
    }

    let mut state = State::new();
    for c in s_hands.chars() {
        state.eat(c)?;
    }

    Ok((Hand(state.counts[0]), Hand(state.counts[1])))
}

fn decode_ply(s_ply: impl AsRef<str>) -> Result<i32> {
    s_ply
        .as_ref()
        .parse::<i32>()
        .map_err(|e| Error::decode_error(format!("ply: parse error: {}", e)))
}

fn tokens_to_moves<'a, I>(tokens: &mut I) -> Result<Vec<Move>>
where
    I: Iterator<Item = &'a str>,
{
    if let Some(magic) = tokens.next() {
        if magic != "moves" {
            return Err(Error::decode_error(r#"moves: "moves" expected"#));
        }
        tokens.map(decode_move).collect::<Result<Vec<_>>>()
    } else {
        Ok(Vec::new())
    }
}

fn decode_move(s_mv: impl AsRef<str>) -> Result<Move> {
    let s_mv = s_mv.as_ref();

    macro_rules! ensure {
        ($cond:expr) => {
            if !$cond {
                return Err(Error::decode_error(format!(
                    "move: invalid string: {}",
                    s_mv
                )));
            }
        };
    }

    let mut cs = ['\0'; 5];
    let mut cs_len = 0;
    for c in s_mv.chars() {
        ensure!(cs_len < 5);
        cs[cs_len] = c;
        cs_len += 1;
    }
    ensure!(cs_len >= 4);

    if cs[1] == '*' {
        ensure!(cs_len == 4);
        let pt = char_to_pt(cs[0])
            .ok_or_else(|| Error::decode_error(format!("move: invalid piece: {}", cs[0])))?;
        let dst = chars_to_sq(cs[2], cs[3])?;
        Ok(Move::drop(pt, dst))
    } else {
        if cs_len == 5 && cs[4] != '+' {
            return Err(Error::decode_error(format!("move: '+' expected: {}", s_mv)));
        }
        let src = chars_to_sq(cs[0], cs[1])?;
        let dst = chars_to_sq(cs[2], cs[3])?;
        let is_promotion = cs_len == 5;
        Ok(Move::nondrop(src, dst, is_promotion))
    }
}

fn chars_to_sq(cx: char, cy: char) -> Result<Square> {
    if !('1'..='9').contains(&cx) {
        return Err(Error::decode_error(format!("square: invalid x: {}", cx)));
    }
    if !('a'..='i').contains(&cy) {
        return Err(Error::decode_error(format!("square: invalid y: {}", cy)));
    }
    let x = cx as u8 - b'1';
    let y = cy as u8 - b'a';
    Ok(Square::new(x, y))
}

fn char_to_side_pt(c: char) -> Option<(Side, PieceType)> {
    let pt = char_to_pt(c.to_ascii_uppercase())?;
    let side = if c.is_ascii_uppercase() {
        Side::Sente
    } else if c.is_ascii_lowercase() {
        Side::Gote
    } else {
        panic!("internal error")
    };
    Some((side, pt))
}

fn char_to_pt(c: char) -> Option<PieceType> {
    match c {
        'P' => Some(PieceType::Pawn),
        'L' => Some(PieceType::Lance),
        'N' => Some(PieceType::Knight),
        'S' => Some(PieceType::Silver),
        'B' => Some(PieceType::Bishop),
        'R' => Some(PieceType::Rook),
        'G' => Some(PieceType::Gold),
        'K' => Some(PieceType::King),
        _ => None,
    }
}
