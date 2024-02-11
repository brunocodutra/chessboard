use crate::chess::*;
use crate::util::{Bits, Integer};
use rand::prelude::*;
use rand_pcg::Pcg64;
use std::mem::MaybeUninit;

/// A type representing a [`Position`]'s [zobrist hash].
///
/// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
pub type Zobrist = Bits<u64, 64>;

#[derive(Debug)]
pub struct ZobristNumbers {
    pieces: [[[u64; 64]; 6]; 2],
    castles: [u64; 16],
    en_passant: [u64; 8],
    turn: u64,
}

static mut ZOBRIST: ZobristNumbers = unsafe { MaybeUninit::zeroed().assume_init() };

#[cold]
#[ctor::ctor]
#[inline(never)]
unsafe fn init() {
    let mut rng = Pcg64::seed_from_u64(0x980E8CE238E3B114);

    ZOBRIST.pieces = rng.gen();
    ZOBRIST.castles = rng.gen();
    ZOBRIST.en_passant = rng.gen();
    ZOBRIST.turn = rng.gen();
}

impl ZobristNumbers {
    #[inline(always)]
    pub fn psq(color: Color, role: Role, sq: Square) -> Zobrist {
        unsafe { Zobrist::new(ZOBRIST.pieces[color as usize][role as usize][sq as usize]) }
    }

    #[inline(always)]
    pub fn castling(castles: Castles) -> Zobrist {
        unsafe { Zobrist::new(ZOBRIST.castles[castles.index() as usize]) }
    }

    #[inline(always)]
    pub fn en_passant(file: File) -> Zobrist {
        unsafe { Zobrist::new(ZOBRIST.en_passant[file as usize]) }
    }

    #[inline(always)]
    pub fn turn() -> Zobrist {
        unsafe { Zobrist::new(ZOBRIST.turn) }
    }
}
