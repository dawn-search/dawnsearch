/*
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::fs;

#[derive(Clone)]
pub struct Config {
    pub config_file: String,

    pub index_cc_enabled: bool,
    pub web_enabled: bool,
    pub web_listen_address: String,

    pub udp_enabled: bool,
    pub udp_listen_address: String,
    pub accept_insert: bool,
    pub upnp_enabled: bool,

    pub trackers: Vec<String>,
    pub data_dir: String,

    pub debug: usize,
}

impl Config {
    pub fn load(config_file: &str) -> Config {
        let mut builder = config::Config::builder();
        let mut used_config_file = "<none>";
        if fs::metadata(&config_file).is_ok() {
            builder = builder.add_source(config::File::with_name(&config_file));
            used_config_file = config_file;
        }
        builder = builder.add_source(config::Environment::with_prefix("DAWNSEARCH"));
        let settings = builder.build().unwrap();

        Config {
            config_file: used_config_file.to_string(),
            index_cc_enabled: settings.get_bool("index_cc").unwrap_or(false),
            web_enabled: settings.get_bool("web").unwrap_or(true),
            web_listen_address: settings
                .get_string("web_listen_address")
                .unwrap_or("0.0.0.0:8080".to_string()),
            udp_enabled: settings.get_bool("udp").unwrap_or(true),
            udp_listen_address: settings
                .get_string("udp_listen_address")
                .unwrap_or("0.0.0.0:8080".to_string()),
            accept_insert: settings.get_bool("accept_insert").unwrap_or(false),
            upnp_enabled: settings.get_bool("upnp").unwrap_or(false),

            trackers: settings
                .get_array("trackers")
                .map(|a| a.iter().map(|v| v.clone().into_string().unwrap()).collect())
                .unwrap_or_default(),
            data_dir: settings.get_string("data_dir").unwrap_or(".".to_string()),
            debug: settings.get_int("debug").unwrap_or(0) as usize,
        }
    }

    pub fn print(&self) {
        println!("==========================================================");
        println!("Config file: {}", self.config_file);
        println!("Indexing Common Crawl enabled: {}", self.index_cc_enabled);
        println!("Web enabled: {}", self.web_enabled);
        println!("Web listen address: {}", self.web_listen_address);
        println!("UDP enabled: {}", self.udp_enabled);
        println!("UDP listen address: {}", self.udp_listen_address);
        println!("UPnP enabled: {}", self.upnp_enabled);
        println!("Trackers: {:?}", self.trackers);
        println!("Data directory: {}", self.data_dir);
        println!("Debug level: {}", self.debug);
        println!("==========================================================");
    }
}
