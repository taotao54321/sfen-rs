use std::borrow::Cow;

use itertools::Itertools;

use crate::*;

pub fn encode(pos: &Position, mvs: &[Move]) -> String {
    let s_pos = encode_pos(pos);
    if mvs.is_empty() {
        s_pos.into_owned()
    } else {
        [s_pos, encode_moves(mvs)].join(" ")
    }
}

fn encode_pos(pos: &Position) -> Cow<'static, str> {
    let s_board = encode_board(pos.board());
    let s_side = encode_side(pos.side());
    let s_hands = encode_hands(pos.hand(Side::Sente), pos.hand(Side::Gote));
    let s_ply = encode_ply(pos.ply());

    ["sfen", &s_board, &s_side, &s_hands, &s_ply]
        .join(" ")
        .into()
}

fn encode_board(board: &Board) -> Cow<'static, str> {
    board.0.chunks(9).map(encode_board_row).join("/").into()
}

fn encode_board_row(row: &[BoardCell]) -> Cow<'static, str> {
    #[derive(Debug)]
    struct State {
        s_row: String,
        n_empty: u32,
        idx: u32,
    }
    impl State {
        fn new() -> Self {
            Self {
                s_row: String::with_capacity(16),
                n_empty: 0,
                idx: 0,
            }
        }
        fn eat(&mut self, cell: &BoardCell) {
            match cell {
                BoardCell::Empty => {
                    self.n_empty += 1;
                }
                BoardCell::Piece(side, pt) => {
                    self.flush_emptys();
                    self.s_row.push_str(&encode_piece(*side, *pt));
                }
            }
            self.idx += 1;
            if self.idx == 9 {
                self.flush_emptys();
            }
        }
        fn flush_emptys(&mut self) {
            if self.n_empty > 0 {
                let c = std::char::from_digit(self.n_empty, 10).expect("internal error");
                self.s_row.push(c);
                self.n_empty = 0;
            }
        }
    }

    let mut state = State::new();
    for cell in row {
        state.eat(cell);
    }

    state.s_row.into()
}

fn encode_side(side: Side) -> Cow<'static, str> {
    match side {
        Side::Sente => "b",
        Side::Gote => "w",
    }
    .into()
}

fn encode_hands(hand_sente: &Hand, hand_gote: &Hand) -> Cow<'static, str> {
    const PTS: [PieceType; 7] = [
        PieceType::Rook,
        PieceType::Bishop,
        PieceType::Gold,
        PieceType::Silver,
        PieceType::Knight,
        PieceType::Lance,
        PieceType::Pawn,
    ];

    if hand_sente.is_empty() && hand_gote.is_empty() {
        return "-".into();
    }

    let mut s_hands = String::with_capacity(16);
    for (side, hand) in [(Side::Sente, hand_sente), (Side::Gote, hand_gote)].iter() {
        for pt in PTS.iter() {
            let n = hand.count(*pt);
            if n == 0 {
                continue;
            }
            if n >= 2 {
                s_hands.push_str(&n.to_string());
            }
            s_hands.push_str(&encode_piece(*side, *pt));
        }
    }

    s_hands.into()
}

fn encode_ply(ply: i32) -> Cow<'static, str> {
    ply.to_string().into()
}

fn encode_moves(mvs: &[Move]) -> Cow<'static, str> {
    std::iter::once("moves".into())
        .chain(mvs.iter().copied().map(encode_move))
        .join(" ")
        .into()
}

fn encode_move(mv: Move) -> Cow<'static, str> {
    fn push_sq(s: &mut String, sq: Square) {
        s.push(char::from(sq.x() + b'1'));
        s.push(char::from(sq.y() + b'a'));
    }

    let mut s_mv = String::with_capacity(5);

    match mv {
        Move::Nondrop(nondrop) => {
            push_sq(&mut s_mv, nondrop.src);
            push_sq(&mut s_mv, nondrop.dst);
            if nondrop.is_promotion {
                s_mv.push('+');
            }
        }
        Move::Drop(drop) => {
            s_mv.push_str(&encode_pt(drop.pt));
            s_mv.push('*');
            push_sq(&mut s_mv, drop.dst);
        }
    }

    s_mv.into()
}

fn encode_piece(side: Side, pt: PieceType) -> Cow<'static, str> {
    let s_pt = encode_pt(pt);
    match side {
        Side::Sente => s_pt,
        Side::Gote => s_pt.to_ascii_lowercase().into(),
    }
}

fn encode_pt(pt: PieceType) -> Cow<'static, str> {
    match pt {
        PieceType::Pawn => "P",
        PieceType::Lance => "L",
        PieceType::Knight => "N",
        PieceType::Silver => "S",
        PieceType::Bishop => "B",
        PieceType::Rook => "R",
        PieceType::Gold => "G",
        PieceType::King => "K",
        PieceType::ProPawn => "+P",
        PieceType::ProLance => "+L",
        PieceType::ProKnight => "+N",
        PieceType::ProSilver => "+S",
        PieceType::Horse => "+B",
        PieceType::Dragon => "+R",
    }
    .into()
}
