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

pub fn input_account_id(language: Language) -> String {
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

fn select_account_id(language: Language, account_ids: Vec<String>) -> String {
    let tip = vec![match language {
        Language::ChineseSimplified => "输入数字选择账号",
        Language::English => "Input a number to select account",
    }
    .to_string()]
    .into_iter()
    .chain(
        account_ids
            .iter()
            .enumerate()
            .map(|(i, account_id)| format!("{}. {}", i + 1, account_id)),
    )
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
                if index > 0 && index <= account_ids.len() {
                    return account_ids[index - 1].clone();
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

pub fn account_id(language: Language, account_ids: Vec<String>) -> String {
    if account_ids.is_empty() {
        let tip = match language {
            Language::ChineseSimplified => "没有账号ID",
            Language::English => "No account ID",
        };
        println!("{}", tip);
        input_account_id(language)
    } else {
        let tip = match language {
            Language::ChineseSimplified => "已有账号ID：",
            Language::English => "Existing account IDs:",
        };
        println!("{}", tip);
        for (i, account_id) in account_ids.iter().enumerate() {
            println!("{}. {}", i + 1, account_id);
        }
        let tip = match language {
            Language::ChineseSimplified => "输入1以选择已有账号，输入2以输入新账号",
            Language::English => {
                "Input 1 to select an existing account, input 2 to input a new account"
            }
        };
        println!("{}", tip);
        loop {
            let mut input = String::new();
            print_input_tip(language);
            stdout().flush().unwrap();
            stdin().read_line(&mut input).unwrap();
            let input = input.trim();
            match input {
                "1" => {
                    return select_account_id(language, account_ids);
                }
                "2" => {
                    return input_account_id(language);
                }
                _ => {
                    print_invalid_input(input.to_string(), language);
                }
            }
        }
    }
}
