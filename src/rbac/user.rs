/*
 * Parseable Server (C) 2022 - 2024 Parseable, Inc.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 */

use std::collections::HashSet;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};

use openid::{CompactJson, CustomClaims, StandardClaims, StandardClaimsSubject};
use rand::distributions::{Alphanumeric, DistString};

use crate::parseable::PARSEABLE;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum UserType {
    Native(Basic),
    OAuth(OAuth),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct User {
    #[serde(flatten)]
    pub ty: UserType,
    pub roles: HashSet<String>,
}

impl User {
    // create a new User and return self with password generated for said user.
    pub fn new_basic(username: String) -> (Self, String) {
        let PassCode { password, hash } = Basic::gen_new_password();
        (
            Self {
                ty: UserType::Native(Basic {
                    username,
                    password_hash: hash,
                }),
                roles: HashSet::new(),
            },
            password,
        )
    }

    pub fn new_oauth(username: String, roles: HashSet<String>, user_info: UserInfo) -> Self {
        Self {
            ty: UserType::OAuth(OAuth {
                userid: user_info.name.clone().unwrap_or(username),
                user_info,
            }),
            roles,
        }
    }

    pub fn username(&self) -> &str {
        match self.ty {
            UserType::Native(Basic { ref username, .. }) => username,
            UserType::OAuth(OAuth {
                userid: ref username,
                ..
            }) => username,
        }
    }

    pub fn is_oauth(&self) -> bool {
        matches!(self.ty, UserType::OAuth(_))
    }

    pub fn roles(&self) -> Vec<String> {
        self.roles.iter().cloned().collect()
    }
}

// Represents a User in the system
// can be the root admin user (set with env vars at startup / restart)
// or user(s) created by the root user
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Basic {
    pub username: String,
    pub password_hash: String,
}

impl Basic {
    // generate a new password
    pub fn gen_new_password() -> PassCode {
        let password = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        let hash = gen_hash(&password);
        PassCode { password, hash }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        verify(&self.password_hash, password)
    }
}

// Take the password and compare with the hash stored internally (PHC format ==>
// $<id>[$v=<version>][$<param>=<value>(,<param>=<value>)*][$<salt>[$<hash>]])
// ref https://github.com/P-H-C/phc-string-format/blob/master/phc-sf-spec.md#specification
pub fn verify(password_hash: &str, password: &str) -> bool {
    let parsed_hash = PasswordHash::new(password_hash).unwrap();
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// generate a one way hash for password to be stored in metadata file
// ref https://github.com/P-H-C/phc-string-format/blob/master/phc-sf-spec.md
fn gen_hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hashcode = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("can hash random alphanumeric")
        .to_string();

    hashcode
}

pub struct PassCode {
    pub password: String,
    pub hash: String,
}

pub fn get_admin_user() -> User {
    let username = PARSEABLE.options.username.clone();
    let password = PARSEABLE.options.password.clone();
    let hashcode = gen_hash(&password);

    User {
        ty: UserType::Native(Basic {
            username,
            password_hash: hashcode,
        }),
        roles: ["admin".to_string()].into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OAuth {
    pub userid: String,
    pub user_info: UserInfo,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    #[serde(default)]
    /// User's full name for display purposes.
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub picture: Option<url::Url>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub groups: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MyClaims {
    pub group: Option<Vec<String>>,
    #[serde(flatten)]
    pub standard_claims: StandardClaims,
}

impl CustomClaims for MyClaims {
    fn standard_claims(&self) -> &StandardClaims {
        &self.standard_claims
    }
}

impl StandardClaimsSubject for MyClaims {
    fn sub(&self) -> Result<&str, openid::error::StandardClaimsSubjectMissing> {
        todo!()
    }
}

impl CompactJson for MyClaims {}

impl From<openid::Userinfo> for UserInfo {
    fn from(user: openid::Userinfo) -> Self {
        UserInfo {
            name: user.name,
            preferred_username: user.preferred_username,
            picture: user.picture,
            email: user.email,
            gender: user.gender,
            updated_at: user.updated_at,
            groups: None,
        }
    }
}

/* impl Into<openid::Userinfo> for StandardClaims {
    fn into(self) -> openid::Userinfo {
        // Conversion logic here
        todo
    }
}
 */
