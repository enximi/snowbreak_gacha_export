use std::io::{stdin, stdout, Write};

use enum_iterator::all;

use crate::language::Language;
use crate::record::BannerType;

fn print_invalid_input(input: String, language: Language) {
    let tip = match language {
        Language::ChineseSimplified => format!("无效输入：{input}，请重新输入"),
        Language::English => format!("Invalid input: {input}, please input again"),
    };
    println!("{}", tip);
}

fn print_input_tip(language: Language) {
    let tip = match language {
        Language::ChineseSimplified => "输入：",
        Language::English => "input: ",
    };
    print!("{}", tip);
}

pub fn language() -> Language {
    fn print_invalid_input(input: String) {
        println!("无效输入：{input}，请重新输入/Invalid input: {input}, please input again");
    }

    let tip = vec!["输入数字选择语言/Input a number to select language".to_string()]
        .into_iter()
        .chain(
            all::<Language>()
                .enumerate()
                .map(|(i, language)| format!("{}. {}", i + 1, language)),
        )
        .collect::<Vec<String>>()
        .join("\n");
    println!("{}", tip);
    loop {
        let mut input = String::new();
        print!("输入/input: ");
        stdout().flush().unwrap();
        stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        match input.parse::<usize>() {
            Ok(index) => {
                if index > 0 && index <= all::<Language>().count() {
                    return all::<Language>().nth(index - 1).unwrap();
                } else {
                    print_invalid_input(input.to_string());
                }
            }
            Err(_) => {
                print_invalid_input(input.to_string());
            }
        }
    }
}

pub fn banner_type(language: Language) -> BannerType {
    let tip = vec![match language {
        Language::ChineseSimplified => "输入数字选择卡池",
        Language::English => "Input a number to select banner",
    }
    .to_string()]
    .into_iter()
    .chain(all::<BannerType>().enumerate().map(|(i, banner_type)| {
        format!("{}. {}", i + 1, banner_type.display_name_for_user(language))
    }))
    .collect::<Vec<String>>()
    .join("\n");
    println!("{}", tip);
    loop {
        let mut input = String::new();
        print_input_tip(language);
        stdout().flush().unwrap();
        stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        match input.parse::<usize>() {
            Ok(index) => {
                if index > 0 && index <= all::<BannerType>().count() {
                    return all::<BannerType>().nth(index - 1).unwrap();
                } else {
                    print_invalid_input(input.to_string(), language);
                }
            }
            Err(_) => {
                print_invalid_input(input.to_string(), language);
            }
        }
    }
}

pub fn account_id(language: Language) -> String {
    loop {
        let tip = match language {
            Language::ChineseSimplified => "输入账号ID：",
            Language::English => "Input account ID: ",
        };
        print!("{}", tip);
        stdout().flush().unwrap();
        let mut account_id = String::new();
        stdin().read_line(&mut account_id).unwrap();
        let account_id = account_id.trim();
        if account_id.is_empty() {
            let tip = match language {
                Language::ChineseSimplified => "账号ID不能为空",
                Language::English => "Account ID cannot be empty",
            };
            println!("{}", tip);
        } else {
            return account_id.to_string();
        }
    }
}

pub fn wait_enter(language: Language) {
    let tip = match language {
        Language::ChineseSimplified => "按下回车键退出",
        Language::English => "Press enter to exit",
    };
    println!("{}", tip);
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
}
