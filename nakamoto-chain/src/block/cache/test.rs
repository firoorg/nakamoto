use super::{BlockCache, BlockTree, Error};

use crate::block::store::{self, Store};
use crate::block::tree::Branch;
use crate::block::{self, Height, Target, Time};

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::iter;
use std::path::Path;
use std::rc::Rc;

use nonempty::NonEmpty;
use quickcheck::{Arbitrary, Gen,QuickCheck};
use quickcheck_macros::quickcheck;
use quickcheck as qc;
use rand::Rng;

use bitcoin::blockdata::block::BlockHeader;
use bitcoin::blockdata::constants;
use bitcoin::consensus::params::Params;
use bitcoin::hash_types::{BlockHash, TxMerkleNode};

use bitcoin::util::hash::BitcoinHash;
use bitcoin::util::uint::Uint256;

// Lowest possible difficulty.
const TARGET: Uint256 = Uint256([
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0x7fffffffffffffffu64,
]);
// Target block time (1 minute).
const TARGET_SPACING: Time = 60;
// Target time span (1 hour).
const _TARGET_TIMESPAN: Time = 60 * 60;

#[derive(Debug)]
pub struct Cache {
    headers: HashMap<BlockHash, BlockHeader>,
    chain: NonEmpty<BlockHeader>,
    tip: BlockHash,
    genesis: BlockHash,
}

impl Cache {
    pub fn new(genesis: BlockHeader) -> Self {
        let mut headers = HashMap::new();
        let hash = genesis.bitcoin_hash();
        let chain = NonEmpty::new(genesis);

        headers.insert(hash, genesis);

        Self {
            headers,
            chain,
            tip: hash,
            genesis: hash,
        }
    }

    fn branch(&self, tip: &BlockHash) -> Option<NonEmpty<BlockHeader>> {
        let mut headers = VecDeque::new();
        let mut tip = *tip;

        while let Some(header) = self.headers.get(&tip) {
            tip = header.prev_blockhash;
            headers.push_front(*header);
        }
        NonEmpty::from_vec(Vec::from(headers))
    }

    fn longest_chain(&self) -> NonEmpty<BlockHeader> {
        let mut branches = Vec::new();

        for tip in self.headers.keys() {
            if let Some(branch) = self.branch(tip) {
                branches.push(branch);
            }
        }

        branches
            .into_iter()
            .max_by(|a, b| Branch(a).work().cmp(&Branch(b).work()))
            .unwrap()
    }
}

impl BlockTree for Cache {
    fn import_blocks<I: Iterator<Item = BlockHeader>>(
        &mut self,
        chain: I,
    ) -> Result<(BlockHash, Height), Error> {
        for header in chain {
            self.headers.insert(header.bitcoin_hash(), header);
        }
        self.chain = self.longest_chain();

        Ok((self.chain.last().bitcoin_hash(), self.height()))
    }

    fn get_block(&self, hash: &BlockHash) -> Option<&BlockHeader> {
        self.headers.get(hash)
    }

    fn get_block_by_height(&self, height: Height) -> Option<&BlockHeader> {
        self.chain.get(height as usize)
    }

    fn tip(&self) -> (BlockHash, BlockHeader) {
        let tip = self.chain.last();
        (tip.bitcoin_hash(), *tip)
    }

    fn height(&self) -> Height {
        self.chain.len() as Height - 1
    }

    fn chain(&self) -> Box<dyn Iterator<Item = (Height, BlockHeader)>> {
        let iter = self
            .chain
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, h)| (i as Height, h));

        Box::new(iter)
    }
}

#[derive(Debug)]
struct HeightCache {
    headers: BTreeMap<Height, BlockHeader>,
    height: Height,
}

impl HeightCache {
    fn new(genesis: BlockHeader) -> Self {
        let mut headers = BTreeMap::new();
        let height = 0;

        headers.insert(height, genesis);

        Self { headers, height }
    }

    fn import(&mut self, height: Height, header: BlockHeader) {
        assert!(height > self.height);
        assert!(!self.headers.contains_key(&height));

        self.headers.insert(height, header);
        self.height = height;
    }
}

impl BlockTree for HeightCache {
    fn import_blocks<I: Iterator<Item = BlockHeader>>(
        &mut self,
        _chain: I,
    ) -> Result<(BlockHash, Height), super::Error> {
        unimplemented!()
    }

    fn get_block(&self, _hash: &BlockHash) -> Option<&BlockHeader> {
        unimplemented!()
    }

    fn get_block_by_height(&self, height: Height) -> Option<&BlockHeader> {
        self.headers.get(&height)
    }

    fn tip(&self) -> (BlockHash, BlockHeader) {
        let header = self.headers.get(&self.height).unwrap();
        (header.bitcoin_hash(), *header)
    }

    fn height(&self) -> Height {
        self.height
    }

    fn chain(&self) -> Box<dyn Iterator<Item = (Height, BlockHeader)>> {
        unimplemented!()
    }
}

mod arbitrary {
    use super::*;

    #[derive(Clone)]
    pub struct OrderedHeaders {
        pub headers: NonEmpty<BlockHeader>,
    }

    impl Arbitrary for OrderedHeaders {
        fn arbitrary<G: Gen>(g: &mut G) -> OrderedHeaders {
            let height = g.gen_range(1, g.size() + 1) as Height;
            Self {
                headers: arbitrary_chain(height, g),
            }
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let Self { headers } = self;
            let mut shrunk = Vec::new();

            if let Some((_, rest)) = headers.tail.split_last() {
                shrunk.push(Self {
                    headers: NonEmpty::from((headers.head, rest.to_vec())),
                });
            }
            Box::new(shrunk.into_iter())
        }
    }

    impl std::fmt::Debug for OrderedHeaders {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(fmt, "\n")?;

            for (height, header) in self.headers.iter().enumerate() {
                writeln!(
                    fmt,
                    "#{:03} {} time={:05} bits={:x} nonce={}",
                    height,
                    header.bitcoin_hash(),
                    header.time,
                    header.bits,
                    header.nonce
                )?;
            }
            Ok(())
        }
    }

    #[derive(Clone)]
    pub struct UnorderedHeaders {
        pub headers: Vec<BlockHeader>,
        pub genesis: BlockHeader,
        pub tip: BlockHash,
    }

    impl UnorderedHeaders {
        fn new(ordered: NonEmpty<BlockHeader>) -> Self {
            let genesis = *ordered.first();
            let tip = ordered.last().bitcoin_hash();
            let headers = ordered.tail.clone();

            UnorderedHeaders {
                headers,
                genesis,
                tip,
            }
        }

        fn shuffle<G: Gen>(&mut self, g: &mut G) {
            use rand::seq::SliceRandom;
            self.headers.shuffle(g);
        }
    }

    impl Arbitrary for UnorderedHeaders {
        fn arbitrary<G: Gen>(g: &mut G) -> UnorderedHeaders {
            let OrderedHeaders { headers: ordered } = OrderedHeaders::arbitrary(g);
            let mut unordered = UnorderedHeaders::new(ordered);

            unordered.shuffle(g);
            unordered
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let mut shrunk = Vec::new();

            if self.tip != self.genesis.bitcoin_hash() {
                let mut unordered = self.clone();
                let ix = unordered
                    .headers
                    .iter()
                    .position(|h| h.bitcoin_hash() == self.tip)
                    .unwrap();
                let tip = unordered.headers[ix];

                unordered.tip = tip.prev_blockhash;
                unordered.headers.swap_remove(ix);

                shrunk.push(unordered);
            }

            Box::new(shrunk.into_iter())
        }
    }

    impl std::fmt::Debug for UnorderedHeaders {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(fmt, "\n")?;

            for header in self.headers.iter() {
                writeln!(
                    fmt,
                    "{} {} time={:05} bits={:x}",
                    header.bitcoin_hash(),
                    header.prev_blockhash,
                    header.time,
                    header.bits,
                )?;
            }
            Ok(())
        }
    }
}

fn arbitrary_header<G: Gen>(
    prev_blockhash: BlockHash,
    prev_time: Time,
    target: &Target,
    g: &mut G,
) -> BlockHeader {
    let delta = g.gen_range(TARGET_SPACING / 2, TARGET_SPACING * 2);

    let time = if delta == 0 {
        prev_time
    } else if delta < prev_time && g.gen_bool(1. / 100.) {
        // Small probability that this block's timestamp is in the past.
        g.gen_range(prev_time.saturating_sub(delta), prev_time)
    } else {
        g.gen_range(prev_time, prev_time + delta)
    };

    let bits = BlockHeader::compact_target_from_u256(&target);

    let mut header = BlockHeader {
        version: 1,
        time,
        nonce: 0,
        bits,
        merkle_root: TxMerkleNode::default(),
        prev_blockhash,
    };

    let target = header.target();
    while header.validate_pow(&target).is_err() {
        header.nonce += 1;
    }

    header
}

fn arbitrary_chain<G: Gen>(height: Height, g: &mut G) -> NonEmpty<BlockHeader> {
    let mut prev_time = 0; // Epoch.
    let mut prev_hash = BlockHash::default();

    let genesis = arbitrary_header(prev_hash, prev_time, &TARGET, g);
    let mut chain = NonEmpty::new(genesis);

    prev_hash = genesis.bitcoin_hash();
    prev_time = genesis.time;

    for _ in 0..height {
        let header = arbitrary_header(prev_hash, prev_time, &TARGET, g);
        prev_time = header.time;
        prev_hash = header.bitcoin_hash();

        chain.push(header);
    }
    chain
}

#[derive(Clone)]
struct BlockImport(BlockCache<store::Memory>, BlockHeader);

impl std::fmt::Debug for BlockImport {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let BlockImport(_, header) = self;
        write!(fmt, "{:#?}", header)
    }
}

impl qc::Arbitrary for BlockImport {
    fn arbitrary<G: Gen>(g: &mut G) -> BlockImport {
        let network = bitcoin::Network::Regtest;
        let genesis = constants::genesis_block(network).header;
        let params = Params::new(network);
        let store = store::Memory::new(NonEmpty::new(genesis));
        let cache = BlockCache::from(store, params).unwrap();
        let header = arbitrary_header(genesis.bitcoin_hash(), genesis.time, &genesis.target(), g);

        cache
            .clone()
            .import_blocks(iter::once(header))
            .expect("the header is valid");

        Self(cache, header)
    }
}

#[quickcheck]
fn prop_block_missing(import: BlockImport) -> bool {
    let BlockImport(mut cache, header) = import;
    let prev_blockhash = constants::genesis_block(bitcoin::Network::Testnet)
        .header
        .bitcoin_hash();

    let header = BlockHeader {
        prev_blockhash,
        ..header
    };

    matches! {
        cache.import_block(header).err(),
        Some(Error::BlockMissing(hash)) if hash == prev_blockhash
    }
}

#[quickcheck]
fn prop_invalid_block_target(import: BlockImport) -> bool {
    let BlockImport(mut cache, header) = import;
    let genesis = cache.genesis().clone();

    assert!(cache.clone().import_block(header).is_ok());

    let header = BlockHeader {
        bits: genesis.bits - 1,
        ..header
    };

    matches! {
        cache.import_block(header).err(),
        Some(Error::InvalidBlockTarget(actual, expected))
            if actual == block::target_from_bits(genesis.bits - 1)
                && expected == genesis.target()
    }
}

#[quickcheck]
fn prop_invalid_block_pow(import: BlockImport) -> bool {
    let BlockImport(mut cache, header) = import;
    let mut header = header.clone();

    // Find an *invalid* nonce.
    while header.validate_pow(&header.target()).is_ok() {
        header.nonce += 1;
    }

    matches! {
        cache.import_block(header).err(),
        Some(Error::InvalidBlockPoW)
    }
}

// Test our difficulty validation against values from the bitcoin main chain.
#[test]
fn test_bitcoin_difficulty() {
    use crate::tests;

    let network = bitcoin::Network::Bitcoin;
    let genesis = constants::genesis_block(network).header;
    let params = Params::new(network);

    let mut cache = HeightCache::new(genesis);

    for (height, prev_time, prev_bits, time, bits) in tests::TARGETS.iter().cloned() {
        let target = cache.next_difficulty_target(height - 1, prev_time, prev_bits, &params);

        assert_eq!(height % params.difficulty_adjustment_interval(), 0);
        assert_eq!(target, bits);

        // We store the retargeting blocks, since they are used in the difficulty calculation.
        cache.import(
            height,
            BlockHeader {
                version: 1,
                time,
                bits,
                merkle_root: Default::default(),
                prev_blockhash: Default::default(),
                nonce: 0,
            },
        );
    }
}

// Test that we're correctly loading headers from the header store.
#[test]
fn test_from_store() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/data/headers.bin");
    let store = store::File::open(path).unwrap();

    let store_headers = store.iter().collect::<Result<Vec<_>, _>>().unwrap();

    let network = bitcoin::Network::Bitcoin;
    let params = Params::new(network);

    let cache = BlockCache::from(store, params).unwrap();
    let cache_headers = cache.chain().collect::<Vec<_>>();

    assert_eq!(store_headers.len(), cache_headers.len());
    assert_eq!(
        store_headers, cache_headers,
        "all stored headers figure in the cache"
    );

    // Make sure all cached headers are also in the `headers` map.
    for (height, header) in store_headers.iter() {
        let result = cache.headers.get(&header.bitcoin_hash());
        assert_eq!(result, Some(height));
    }
}

#[test]
fn prop_cache_import_ordered() {
    fn prop(input: arbitrary::OrderedHeaders) -> bool {
        let arbitrary::OrderedHeaders { headers } = input;
        let mut cache = Cache::new(headers.head);
        let tip = *headers.last();

        cache.import_blocks(headers.tail.iter().cloned()).unwrap();

        cache.genesis() == &headers.head
            && cache.tip() == (tip.bitcoin_hash(), tip)
            && cache
                .chain()
                .all(|(i, h)| headers.get(i as usize) == Some(&h))
    }
    QuickCheck::with_gen(qc::StdGen::new(rand::thread_rng(), 16))
        .quickcheck(prop as fn(arbitrary::OrderedHeaders) -> bool);
}

#[derive(Debug)]
struct Tree {
    headers: Rc<RefCell<HashMap<BlockHash, BlockHeader>>>,
    genesis: BlockHeader,
    hash: BlockHash,
    time: Time,
}

impl Tree {
    fn new(genesis: BlockHeader) -> Self {
        let headers = HashMap::new();
        let hash = genesis.bitcoin_hash();

        Self {
            headers: Rc::new(RefCell::new(headers)),
            time: genesis.time,
            genesis,
            hash,
        }
    }

    fn next(&self) -> Tree {
        let mut header = BlockHeader {
            version: 1,
            prev_blockhash: self.hash,
            merkle_root: Default::default(),
            bits: BlockHeader::compact_target_from_u256(&TARGET),
            time: self.time + TARGET_SPACING,
            nonce: 0,
        };
        self.solve(&mut header);

        self.headers
            .borrow_mut()
            .insert(header.bitcoin_hash(), header);

        Tree {
            headers: self.headers.clone(),
            hash: header.bitcoin_hash(),
            time: header.time,
            genesis: self.genesis,
        }
    }

    fn headers(&self) -> Vec<BlockHeader> {
        self.headers.borrow().values().cloned().collect()
    }

    fn solve(&self, header: &mut BlockHeader) {
        let target = header.target();
        while header.validate_pow(&target).is_err() {
            header.nonce += 1;
        }
    }
}

#[test]
#[allow(unused_variables)]
fn test_cache_import_unordered() {
    let genesis = BlockHeader {
        version: 1,
        prev_blockhash: Default::default(),
        merkle_root: Default::default(),
        bits: BlockHeader::compact_target_from_u256(&TARGET),
        nonce: 0,
        time: 0,
    };
    let a0 = Tree::new(genesis);

    // a0 <- a1 <- a2 <- a3 *
    let a1 = a0.next();
    let a2 = a1.next();
    let a3 = a2.next();

    // a0 <- a1 <- a2 <- a3 *
    //          <- b2 <- b3
    let b2 = a1.next();
    let b3 = b2.next();

    // a0 <- a1 <- a2 <- a3
    //          <- b2 <- b3 <- b4 *
    let b4 = b3.next();

    let mut cache = Cache::new(genesis);
    cache.import_blocks(a0.headers().into_iter()).unwrap();

    let actual = cache
        .chain()
        .map(|(_, h)| h.bitcoin_hash())
        .collect::<Vec<_>>();
    let expected = vec![a0.hash, a1.hash, b2.hash, b3.hash, b4.hash];

    assert_eq!(actual, expected);
}
