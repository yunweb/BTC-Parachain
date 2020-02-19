#![cfg_attr(not(feature = "std"), no_std)]

/// For more guidance on FRAME pallets, see the example.
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs

/// # BTC-Relay implementation
/// This is the implementation of the BTC-Relay following the spec at:
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure};
use {system::ensure_signed, timestamp};
use sp_core::{U256, H256, H160};
use bitcoin::{RawBlockHeader, BlockHeader, BlockChain, parse_block_header};

/// ## Configuration and Constants
/// The pallet's configuration trait.
/// For further reference, see: 
/// https://interlay.gitlab.io/polkabtc-spec/btcrelay-spec/spec/data-model.html
pub trait Trait: timestamp::Trait + system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    
    /// Difficulty Adjustment Interval
    const DIFFICULTY_ADJUSTMENT_INTERVAL: u16 = 2016;

    /// Target Timespan
    const TARGET_TIMESPAN: u64 = 1209600;
    
    /// Unrounded Maximum Target
    /// 0x00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
    const UNROUNDED_MAX_TARGET: U256 = U256([0x00000000ffffffffu64, <u64>::max_value(), <u64>::max_value(), <u64>::max_value()]);
}

// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as BTCRelay {
    /// ## Storage
        /// Store Bitcoin block headers
        BlockHeaders get(fn blockheader): map H256 => BlockHeader<U256, H256, T::Moment>;
        
        // TODO: Chains implementation with priority queue
        /// Priority queue of BlockChain elements
        Chains get(fn chain): Vec<BlockChain<U256, H256>>;

        /// Store the index for each tracked blockchain
        ChainsIndex get(fn chainindex): map U256 => BlockChain<U256, H256>;
        
        /// Store the current blockchain tip
        BestBlock get(fn bestblock): H256;

        /// Store the height of the best block
        BestBlockHeight get(fn bestblockheight): U256;

        /// Track existing BlockChain entries
        ChainCounter get(fn chaincounter): U256;
	}
}

// The pallet's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;
        
        // Initialize errors
        type Error = Error<T>;

        fn initialize(
            origin,
            block_header_bytes: Vec<u8>,
            block_height: U256) 
            -> DispatchResult
        {
            let sender = ensure_signed(origin)?;
            
            // Check if BTC-Relay was already initialized
            let bestblock = Self::bestblock();
            ensure!(bestblock.is_zero(), Error::<T>::AlreadyInitialized);

            // Parse the block header bytes to extract the required info
            let mut block_header = parse_block_header(block_header_bytes, block_height);

            // get a new chain id
            let chain_id: U256 = Self::increment_chain_counter(); 
            // set the chain id as a reference in the block_header
            block_header.chain_ref = Some(chain_id);

            // Store a new BlockHeader struct in BlockHeaders
            Self::store_main_header(block_header);

            // construct the BlockChain struct
            // TODO: change this to a mapping
            let mut chain: Vec<H256> = Vec::new();
            chain.push(hash_current_block);
            let blockchain = BlockChain {
                chain_id: chain_id,
                chain: chain,
                max_height: block_height,
                no_data: false,
                invalid: false,
            };
            // Insert a pointer to BlockChain in ChainsIndex
            <ChainsIndex>::insert(chain_id, &blockchain); 
           
            // Store the new BlockChain in Chains
            let mut vec_blockchain: Vec<BlockChain<U256, H256>> = Vec::new();
            vec_blockchain.push(blockchain);
  
            <Chains>::put(vec_blockchain);

            // Set BestBlock and BestBlockHeight to the submitted block
            <BestBlockHeight>::mutate(|n| *n = block_height);
            // Emit a Initialized Event
            Self::deposit_event(RawEvent::Initialized(block_height, hash_current_block));
            
            Ok(())
        }
    
        fn store_block_header(origin, block_header_bytes: Vec<u8>)
        -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: Check if BTC _Parachain is in shutdown state.

            // TODO: call verify_block_header
            
            // TODO: call the parse block header function
            // Parse the block header bytes to extract the required info
            let merkle_root: H256 = H256::zero();
            let timestamp: T::Moment = <timestamp::Module<T>>::get();
            let target: U256 = U256::max_value();
            let hash_current_block: H256 = H256::zero();
            let hash_prev_block: H256 = H256::zero();
            
            // get the previous block header by the hash
            let prev_block_header = Self::blockheader(hash_prev_block); 
            
            // get the block chain of the previous header
            let blockchain = Self::chainindex(prev_block_header.chain_ref);
            
            // check if the prev block is the highest block in the chain
            // extend the chain or store a new fork
            match prev_block_header.block_height {
                blockchain.max_height => store_main_header(), 
                _ => store_fork_header(),
            }
            
            


            Ok(())
        }
	}
}

/// Utility functions
impl<T: Trait> Module<T> {
    fn increment_chain_counter() -> U256 {
        let new_counter = <ChainCounter>::get() + 1;
        <ChainCounter>::put(new_counter);

        return new_counter;
    }
    fn store_main_header(block_hash: H256, block_header: BlockHeader) {
        <BlockHeaders<T>>::insert(block_hash, block_header);
    }
    fn store_fork_header()
}

decl_event! {
	pub enum Event<T> where 
        AccountId = <T as system::Trait>::AccountId,
        H256 = H256,
        U256 = U256,
    {
        Initialized(U256, H256),
        StoreMainChainHeader(U256, H256),
        StoreForkHeader(U256, U256, H256),
        ChainReorg(H256, U256, U256),
        VerifyTransaction(H256, U256, U256),
        ValidateTransaction(H256, U256, H160, H256),
		SomethingStored(u32, AccountId),
	}
}

// TODO: how to include message in errors?
decl_error! {
    pub enum Error for Module<T: Trait> {
        AlreadyInitialized,
        NotMainChain,
        ForkPrevBlock,
        NotFork,
        InvalidForkId,
        InvalidHeaderSize,
        DuplicateBlock,
        PrevBlock,
        LowDiff,
        DiffTargetHeader,
        MalformedTxid,
        Confirmations,
        InvalidMerkleProof,
        ForkIdNotFound,
        Partial,
        Invalid,
        Shutdown,
        InvalidTxid,
        InsufficientValue,
        TxFormat,
        WrongRecipient,
        InvalidOpreturn,
        InvalidTxVersion,
        NotOpReturn,
    }
}


/// tests for this pallet
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the pallet, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
	}
	impl Trait for Test {
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		new_test_ext().execute_with(|| {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
