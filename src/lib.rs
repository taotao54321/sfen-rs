mod decode;
mod encode;

pub use decode::decode;
pub use encode::encode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sfen decode error: {0}")]
    DecodeError(String),
}

impl Error {
    fn decode_error(msg: impl Into<String>) -> Self {
        Self::DecodeError(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Sente = 0,
    Gote,
}

fn xy2idx(x: u8, y: u8) -> usize {
    (9 * y + x) as usize
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Square(u8);

impl Square {
    #[rustfmt::skip]
    const SQ_TO_X: [u8; 81] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
        0, 1, 2, 3, 4, 5, 6, 7, 8,
    ];

    #[rustfmt::skip]
    const SQ_TO_Y: [u8; 81] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 2,
        3, 3, 3, 3, 3, 3, 3, 3, 3,
        4, 4, 4, 4, 4, 4, 4, 4, 4,
        5, 5, 5, 5, 5, 5, 5, 5, 5,
        6, 6, 6, 6, 6, 6, 6, 6, 6,
        7, 7, 7, 7, 7, 7, 7, 7, 7,
        8, 8, 8, 8, 8, 8, 8, 8, 8,
    ];

    /// 筋 x, 段 y のマスを返す。
    /// x は 0..9 で、0 が1筋。
    /// y は 0..9 で、0 が1段目。
    ///
    /// x または y が範囲外の場合、panic する。
    pub fn new(x: u8, y: u8) -> Self {
        if x >= 9 || y >= 9 {
            panic!("square out of range: ({}, {})", x, y);
        }
        Self(xy2idx(x, y) as u8)
    }

    pub fn x(&self) -> u8 {
        Self::SQ_TO_X[self.0 as usize]
    }

    pub fn y(&self) -> u8 {
        Self::SQ_TO_Y[self.0 as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PieceType {
    Pawn = 0,
    Lance,
    Knight,
    Silver,
    Bishop,
    Rook,
    Gold,
    King,
    ProPawn,
    ProLance,
    ProKnight,
    ProSilver,
    Horse,
    Dragon,
}

impl PieceType {
    fn is_hand(&self) -> bool {
        matches!(
            self,
            Self::Pawn
                | Self::Lance
                | Self::Knight
                | Self::Silver
                | Self::Bishop
                | Self::Rook
                | Self::Gold
        )
    }

    fn to_promoted(&self) -> Option<Self> {
        match self {
            Self::Pawn => Some(Self::ProPawn),
            Self::Lance => Some(Self::ProLance),
            Self::Knight => Some(Self::ProKnight),
            Self::Silver => Some(Self::ProSilver),
            Self::Bishop => Some(Self::Horse),
            Self::Rook => Some(Self::Dragon),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoardCell {
    Empty,
    Piece(Side, PieceType),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board([BoardCell; 81]);

impl Board {
    /// f(x: u8, y: u8) -> BoardCell を用いて初期化した盤面を返す。
    ///
    /// 合法性チェックは一切行わない。
    pub fn new<F>(mut f: F) -> Self
    where
        F: FnMut(u8, u8) -> BoardCell,
    {
        let mut cells = [BoardCell::Empty; 81];
        for y in 0..9 {
            for x in 0..9 {
                cells[xy2idx(x, y)] = f(x, y);
            }
        }
        Self(cells)
    }

    pub fn at(&self, x: u8, y: u8) -> BoardCell {
        self.0[xy2idx(x, y)]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hand([u8; 7]);

impl Hand {
    const PTS: [PieceType; 7] = [
        PieceType::Pawn,
        PieceType::Lance,
        PieceType::Knight,
        PieceType::Silver,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Gold,
    ];

    /// f(pt: PieceType) -> u8 を用いて初期化した持駒を返す。
    ///
    /// 合法性チェックは一切行わない。
    pub fn new<F>(mut f: F) -> Self
    where
        F: FnMut(PieceType) -> u8,
    {
        let mut counts = [0; 7];
        for &pt in Self::PTS.iter() {
            counts[pt as usize] = f(pt);
        }
        Self(counts)
    }

    pub fn count(&self, pt: PieceType) -> u8 {
        self.0[pt as usize]
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (PieceType, u8)> + '_ {
        Self::PTS.iter().map(move |&pt| (pt, self.count(pt)))
    }

    fn empty() -> Self {
        Self([0; 7])
    }

    fn is_empty(&self) -> bool {
        self.0.iter().all(|&n| n == 0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    side: Side,
    board: Board,
    hands: [Hand; 2],
    ply: i32,
}

impl Position {
    pub fn new(side: Side, board: Board, hand_sente: Hand, hand_gote: Hand, ply: i32) -> Self {
        Self {
            side,
            board,
            hands: [hand_sente, hand_gote],
            ply,
        }
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn hand(&self, side: Side) -> &Hand {
        &self.hands[side as usize]
    }

    pub fn ply(&self) -> i32 {
        self.ply
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveNondrop {
    src: Square,
    dst: Square,
    is_promotion: bool,
}

impl MoveNondrop {
    pub fn src(&self) -> Square {
        self.src
    }

    pub fn dst(&self) -> Square {
        self.dst
    }

    pub fn is_promotion(&self) -> bool {
        self.is_promotion
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveDrop {
    pt: PieceType,
    dst: Square,
}

impl MoveDrop {
    pub fn pt(&self) -> PieceType {
        self.pt
    }

    pub fn dst(&self) -> Square {
        self.dst
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Move {
    Nondrop(MoveNondrop),
    Drop(MoveDrop),
}

impl Move {
    pub fn nondrop(src: Square, dst: Square, is_promotion: bool) -> Self {
        Self::Nondrop(MoveNondrop {
            src,
            dst,
            is_promotion,
        })
    }

    pub fn drop(pt: PieceType, dst: Square) -> Self {
        Self::Drop(MoveDrop { pt, dst })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(sfen_from: impl AsRef<str>) -> Result<()> {
        let sfen_from = sfen_from.as_ref();
        let (pos, mvs) = decode(sfen_from)?;
        let sfen_to = encode(&pos, &mvs);
        assert_eq!(sfen_from, sfen_to);
        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        assert_eq!(
            decode("startpos")?,
            decode("sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1")?
        );

        roundtrip("sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1")?;
        roundtrip(
            "sfen 8l/1l+R2P3/p2pBG1pp/kps1p4/Nn1P2G2/P1P1P2PP/1PS6/1KSG3+r1/LN2+p3L w Sbgn3p 1",
        )?;
        roundtrip("sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1 moves 7g7f 3c3d 8h2b+ 3a2b B*4e B*8e 4e3d 8e7f")?;

        Ok(())
    }
}
