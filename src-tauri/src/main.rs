// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod kvcache;
mod model;
mod operators;
mod params;
mod tensor;
use kvcache::KVCache;
use rand::Rng;

use core::fmt;
use std::{alloc::System, path::PathBuf};
use model::Llama;
use rand::random;
use tokenizers::Tokenizer;

use std::time::Duration;
use std::thread;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

struct Pair<T, U> {
  first: T,
  second: U,
}


lazy_static! {
  static ref QUESTION_MAP: Arc<Mutex<HashMap<String, Pair<String,String>>>> = {
    Arc::new(Mutex::new(HashMap::new()))
  };
  static ref ANSWER_MAP: Arc<Mutex<HashMap<String, String>>> = {
    Arc::new(Mutex::new(HashMap::new()))
  };

  static ref RESET_MAP: Arc<Mutex<HashMap<String, String>>> = {
    Arc::new(Mutex::new(HashMap::new()))
  };

  static ref CACHE_MAP: Arc<Mutex<HashMap<String, Pair<Vec<Pair<String, usize>>, kvcache::KVCache<f32>>>>> = {
    Arc::new(Mutex::new(HashMap::new()))
  };
}

lazy_static! {
  static ref LLAMACOM: Arc< model::Llama::<f32>> = {
    println!("start!");
    let project_dir = env!("CARGO_MANIFEST_DIR");
    println!("project_dir: {project_dir}");
    let model_dir = PathBuf::from(project_dir).join("models").join("chat");
    println!("load Llama");
    Arc::new(model::Llama::<f32>::from_safetensors(&model_dir))
  };

  static ref TOKENIZER: Arc<Tokenizer> = {
    let project_dir = env!("CARGO_MANIFEST_DIR");
    let model_dir = PathBuf::from(project_dir).join("models").join("chat");
    Arc::new(Tokenizer::from_file(model_dir.join("tokenizer.json")).unwrap())
  };
}

// 接受参数
#[tauri::command(rename_all = "snake_case")]
fn deal_question(question: &str, name: &str, id: String) -> String {
  println!("前端传过来的问题: {}, {}, {}", question, name, id);
  let mut s: std::sync::MutexGuard<'_, HashMap<String, Pair<String, String>>> = QUESTION_MAP.lock().unwrap();
  s.insert(name.into(), Pair { first: id.clone(), second: question.into() });
  "".into()
}

// 删除
#[tauri::command(rename_all = "snake_case")]
fn reset_question( name: String, id: String) -> String {
  println!("重新设置的传过来的问题: {}, {}", name, id);

  let mut q = RESET_MAP.lock().unwrap();
  q.insert(name, id);
  "".into()
}

// 返回答案
#[tauri::command(rename_all = "snake_case")]
fn send_answer(name: String) -> String {
  //let fake_answer = vec!["abcd", "1234"];
  let mut answer_map =  ANSWER_MAP.lock().unwrap();
 // let tmp = answer_map.get(&name);
  println!("answer map size:{}", answer_map.len());
  match answer_map.remove(&name) {
      Some(s) => {
        println!("{}  answer is {}", name, s);
        s.clone()
      },
      // None =>  fake_answer[rand::thread_rng().gen_range(0..fake_answer.len())].into(),
       None =>  "".into(),
  }
  // s
}

fn infer(name: String, input: String, id: String) {
  println!("{name}, into infer");
  let mut cache_map = CACHE_MAP.lock().unwrap();
  let mut vec_cache = &mut cache_map.get_mut(&name).unwrap();
  let mut kvcache = &mut vec_cache.second;
  vec_cache.first.push(Pair { first: id.clone(), second: kvcache.len() });
  let new_input = format!("<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant", input.trim());
  let binding = TOKENIZER.encode(new_input, true).unwrap();
  let input_ids = binding.get_ids();
  println!("{name}, start infer answer");
  let output_ids = LLAMACOM.chat_generate(input_ids, kvcache, 500, 0.9, 4, 1.);
  let answer = TOKENIZER.decode(&output_ids, true).unwrap();
  let mut answer_map = ANSWER_MAP.lock().unwrap();
  answer_map.insert(name.clone(), answer);
  println!("infer {name}:question :{}  len: {}", id, kvcache.len()); 
}

fn reset_cache(name: String, id: String) {
  let mut cache_map = CACHE_MAP.lock().unwrap();
  let mut vec_cache = &mut cache_map.get_mut(&name).unwrap();
  println!("{name}, start reset");
  let mut index = 0;
  while vec_cache.first.is_empty() == false && index == 0 {
      match  vec_cache.first.pop() {
        Some(tmp) => { 
          if tmp.first == id  {
            index = tmp.second;
            break;
          }
        }
        None => break,
      }
  }
  print!("reset: {}, len to {}", name, index);
  vec_cache.second.reset_len(index);

}

fn accept() {
  println!("tokenizer!");
  let  input = "<|im_start|>system
  You are a highly knowledgeable and friendly assistant. Your goal is to understand and respond to user inquiries with clarity. Your interactions are always respectful, helpful, and focused on delivering the most accurate information to the user.<|im_end|>
<|im_start|>user
  Hey! Got a question for you!<|im_end|>
<|im_start|>assistant";
  let mut tmp_kvcahce = LLAMACOM.new_cache();
  let binding = TOKENIZER.encode(input, true).unwrap();
  let input_ids = binding.get_ids();
  LLAMACOM.chat_generate(input_ids, &mut tmp_kvcahce, 500, 0.9, 4, 1.);
  loop {
    thread::sleep(Duration::from_secs(3));
    let mut cache_map = CACHE_MAP.lock().unwrap();
    let mut question_map = QUESTION_MAP.lock().unwrap();
    let mut reset_map = RESET_MAP.lock().unwrap();
    let reset_key: Vec<String> = reset_map
    .iter()
    .map(
      |(name, id)| {
        let name_clone = name.clone();
        let id_clone = id.clone();
        thread::spawn(move || reset_cache(name_clone, id_clone));
        name.clone()
      }
    ).collect();

    for key in reset_key {
      reset_map.remove(&key);
    }

    let keys_to_remove: Vec<String> = question_map

        .iter()  
        .map(|(k, _value)| {
            if !cache_map.contains_key(k) {
              cache_map.insert(k.clone(), Pair { first: Vec::new(), second: tmp_kvcahce.clone() });
            } 
            let t = k.clone();
            let id= _value.first.clone();
            let v = _value.second.clone();
            thread::spawn(move || infer(t, v, id));
          k.clone()
        })
        .collect();

    // 2. 遍历并删除这些键
    for key in keys_to_remove {
      question_map.remove(&key);
    }
    //println!("accept question");
  }
}

fn main() {

  thread::spawn(move || accept());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, deal_question, send_answer, reset_question])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
