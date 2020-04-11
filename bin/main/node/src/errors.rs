// Copyright 2019, 2020 Wingchain
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

use error_chain::*;

error_chain! {
	foreign_links {
		IO(std::io::Error) #[doc="IO error"];
	}
	links {
		Base(base::errors::Error, base::errors::ErrorKind) #[doc="Base error"];
		Service(service::errors::Error, service::errors::ErrorKind) #[doc="Service error"];
	}
	errors {
		HomeDirNotInited(path: String) {
			description(""),
			display("Home dir not inited: {}", path),
		}
	}
}
