#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
    codec::{Decode, Encode},
    dispatch::Vec,
    sp_runtime::RuntimeDebug,
};
use scale_info::TypeInfo;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Votes {
    Yes,
    No,
}

/// Election information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ElectionInfo<AccountId> {
    all_candidates: Vec<AccountId>,
    voters_list: Vec<AccountId>,
    status: bool,
    winner: Option<AccountId>,
}

/// Candidate information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CandidateInfo<AccountId, Hash> {
    election_hash: Hash,
    total_aye_votes: Vec<AccountId>,
    total_naye_votes: Vec<AccountId>,
}

// Pallet definition
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<T>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::storage]
    pub type Election<T: Config> =
        StorageMap<_, Blake2_128Concat, T::Hash, ElectionInfo<T::AccountId>>;

    #[pallet::storage]
    pub type Candidate<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, CandidateInfo<T::AccountId, T::Hash>>;

    #[pallet::storage]
    pub type CandidatesList<T: Config> = StorageValue<_, Vec<T::AccountId>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CandidateRequestRaised { candidate: T::AccountId },
        ElectionStarted { election_info: ElectionInfo<T::AccountId> },
        CastVote { candidate: T::AccountId, response: Votes },
        Winner { election_hash: T::Hash, election_info: ElectionInfo<T::AccountId> },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        UserAlreadyPresent,
        ElectionAlreadyStarted,
        AlreadyApproved,
        InactiveElection,
        InvalidElectionHash,
        InvalidCandidateChosen,
        DuplicateVoteNotAllowed,
        CandidateNotAvailable,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn raise_join_request_as_candidate(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut all_candidates = CandidatesList::<T>::get();
            ensure!(!all_candidates.contains(&who), Error::<T>::UserAlreadyPresent);

            all_candidates.push(who.clone());
            CandidatesList::<T>::put(all_candidates);

            Self::deposit_event(Event::<T>::CandidateRequestRaised { candidate: who });

            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn start_election(
            origin: OriginFor<T>,
            election_hash: T::Hash,
            election: ElectionInfo<T::AccountId>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let is_available = Election::<T>::contains_key(&election_hash);
            ensure!(!is_available, Error::<T>::ElectionAlreadyStarted);

            Election::<T>::insert(&election_hash, &election);

            Self::deposit_event(Event::<T>::ElectionStarted { election_info: election });

            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn cast_vote(
            origin: OriginFor<T>,
            candidate: T::AccountId,
            election_hash: T::Hash,
            vote: Votes,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut election = Election::<T>::get(&election_hash).ok_or(Error::<T>::InvalidElectionHash)?;

            ensure!(
                election.all_candidates.contains(&candidate),
                Error::<T>::InvalidCandidateChosen
            );

            ensure!(
                !election.voters_list.contains(&who),
                Error::<T>::DuplicateVoteNotAllowed
            );

            election.voters_list.push(who.clone());
            Election::<T>::insert(&election_hash, election.clone());

            let already_initiated = Candidate::<T>::contains_key(&candidate);

            if already_initiated {
                let mut candidate_detail = Candidate::<T>::get(&candidate)
                    .ok_or(Error::<T>::InvalidCandidateChosen)?;

                // Update vote counts for the candidate
                candidate_detail.total_aye_votes.push(who.clone());

                Candidate::<T>::insert(&candidate, candidate_detail);
            } else {
                let candidate_info = CandidateInfo {
                    election_hash: election_hash.clone(),
                    total_aye_votes: vec![who.clone()],
                    total_naye_votes: Vec::new(),
                };

                Candidate::<T>::insert(&candidate, candidate_info);
            }

            Self::deposit_event(Event::<T>::CastVote { candidate, response: vote });

            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn stop_election(origin: OriginFor<T>, election_hash: T::Hash) -> DispatchResult {
            ensure_root(origin)?;

            let mut election =
                Election::<T>::get(&election_hash).ok_or(Error::<T>::InvalidElectionHash)?;

            ensure!(election.status, Error::<T>::InactiveElection);
            election.status = false;

            // Calculate the winner
            let all_candidates = &election.all_candidates;
            let mut vote_count = 0;

            for i in all_candidates {
                let vote_info = Candidate::<T>::get(&i).ok_or(Error::<T>::CandidateNotAvailable)?;
                let aye_vote_count = vote_info.total_aye_votes.len();

                if aye_vote_count > vote_count {
                    vote_count = aye_vote_count;
                    election.winner = Some(i.clone());
                }
            }

            Election::<T>::insert(&election_hash, &election);

            Self::deposit_event(Event::<T>::Winner { election_hash, election_info: election });

            Ok(())
        }
    }
}
