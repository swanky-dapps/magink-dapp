// Copyright 2018-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod link {
    use ink_prelude::vec::Vec;
    use ink_storage::{
        traits::SpreadAllocate,
        Mapping,
    };

    type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Link {
        /// Slug -> URL
        urls: Mapping<Vec<u8>, Vec<u8>>,
        /// URL -> Slug
        slugs: Mapping<Vec<u8>, Vec<u8>>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        /// The slug is already in use for another URL.
        SlugUnavailable,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Slug {
        New(Vec<u8>),
        DeduplicateOrNew(Vec<u8>),
        Deduplicate,
    }

    impl Slug {
        fn get(&self) -> Option<&[u8]> {
            match self {
                Slug::New(slug) | Slug::DeduplicateOrNew(slug) => Some(slug),
                Slug::Deduplicate => None,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum ShorteningOutcome {
        Shortened,
        Deduplicated { slug: Vec<u8> },
        UrlNotFound,
    }

    #[ink(event)]
    pub struct Shortened {
        slug: Vec<u8>,
        url: Vec<u8>,
    }

    #[ink(event)]
    pub struct Deduplicated {
        slug: Vec<u8>,
        url: Vec<u8>,
    }

    #[ink(event)]
    pub struct SlugUnavailable {
        slug: Vec<u8>,
    }

    #[ink(event)]
    pub struct UrlNotFound {
        url: Vec<u8>,
    }

    impl Link {
        #[ink(constructor)]
        pub fn default() -> Self {
            ink_lang::utils::initialize_contract(|_contract: &mut Self| {})
        }

        #[ink(message)]
        pub fn shorten(&mut self, slug: Slug, url: Vec<u8>) -> Result<ShorteningOutcome> {
            // Prevent duplicate slugs
            if let Some(slug) = slug
                .get()
                .and_then(|slug| self.urls.get(slug).map(|_| slug))
            {
                self.env().emit_event(SlugUnavailable {
                    slug: slug.to_vec(),
                });
                return Err(Error::SlugUnavailable)
            }

            // Deduplicate if requested by the user
            let slug = match (slug, self.slugs.get(&url)) {
                (Slug::Deduplicate | Slug::DeduplicateOrNew(_), Some(slug)) => {
                    self.env().emit_event(Deduplicated {
                        slug: slug.clone(),
                        url,
                    });
                    return Ok(ShorteningOutcome::Deduplicated { slug })
                }
                (Slug::Deduplicate, None) => {
                    self.env().emit_event(UrlNotFound { url });
                    return Ok(ShorteningOutcome::UrlNotFound)
                }
                (Slug::New(slug) | Slug::DeduplicateOrNew(slug), _) => slug,
            };

            // No dedup: Insert new slug
            self.urls.insert(&slug, &url);
            self.slugs.insert(&url, &slug);
            self.env().emit_event(Shortened {
                slug: slug.clone(),
                url,
            });
            Ok(ShorteningOutcome::Shortened)
        }

        #[ink(message)]
        pub fn resolve(&self, slug: Vec<u8>) -> Option<Vec<u8>> {
            self.urls.get(slug)
        }
    }
}
