use rand::{RngExt, seq::IndexedRandom};

static MALE_FIRST_NAMES: &[&str] = &["Александр", "Дмитрий", "Максим", "Никита", "Илья", "Артем", "Иван"];
static FEMALE_FIRST_NAMES: &[&str] = &["Анна", "Мария", "Дарья", "Анастасия", "Екатерина", "Елена"];

static LAST_NAMES: &[&str] = &[
  "Иванов", "Петров", "Смирнов", "Кузнецов", "Белый", "Спасский", "Новиков", "Чайковский", "Григорьев", "Волков",
  "Соловьев", "Морозов", "Лебедев", "Козлов", "Степанов", "Павлов", "Семенов", "Григорьев", "Васильев", "Михайлов",
  "Тверской", "Московский"
];

static MALE_PATRONYMICS: &[&str] = &["Александрович", "Дмитриевич", "Сергеевич", "Иванович", "Петрович", "Максимович", "Никитич", "Ильич", "Артемович", "Михайлович"];
static FEMALE_PATRONYMICS: &[&str] = &["Александровна", "Дмитриевна", "Сергеевна", "Ивановна", "Петровна", "Максимовна", "Артемовна", "Михайловна"];

/// Генерирует случайное имя
pub fn generate_random_name() -> String
{
  let mut final_name = String::new();
  let mut rng = rand::rng();

  let is_male = rng.random_bool(0.5);

  let first_name = if is_male {
    MALE_FIRST_NAMES.choose(&mut rng).unwrap()
  } else {
    FEMALE_FIRST_NAMES.choose(&mut rng).unwrap()
  };

  if rng.random_bool(0.8) {
    final_name.push_str(first_name);
  }

  if rng.random_bool(0.4) {
    final_name.push_str(" ");
    let raw_last_name = LAST_NAMES.choose(&mut rng).unwrap();
    let last_name = if is_male {
      raw_last_name.to_string()
    } else {
      decline_last_name_female(raw_last_name)
    };
    final_name.push_str(last_name.as_str());
  }

  if rng.random_bool(0.2) {
    final_name.push_str(" ");
    let patronymic = if is_male {
      MALE_PATRONYMICS.choose(&mut rng).unwrap()
    } else {
      FEMALE_PATRONYMICS.choose(&mut rng).unwrap()
    };
    final_name.push_str(patronymic);
  }

  if final_name.is_empty() {
    final_name.push_str(first_name);
  }

  final_name.trim().to_string()
}

fn decline_last_name_female(last_name: &str) -> String
{
  if last_name.ends_with("ов") || last_name.ends_with("ев") {
    format!("{}а", last_name)
  } else if last_name.ends_with("ский") {
    format!("{}ая", &last_name[..last_name.len() - 2])
  } else {
    last_name.to_string()
  }
}