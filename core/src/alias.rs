use rand::Rng;

/// Adjectives for random alias generation (English)
const ADJECTIVES_EN: &[&str] = &[
    "Adorable", "Beautiful", "Big", "Bright", "Clean",
    "Clever", "Cool", "Cute", "Cunning", "Determined",
    "Energetic", "Efficient", "Fantastic", "Fast", "Fine",
    "Fresh", "Good", "Gorgeous", "Great", "Handsome",
    "Hot", "Kind", "Lovely", "Mystic", "Neat",
    "Nice", "Patient", "Pretty", "Powerful", "Rich",
    "Secret", "Smart", "Solid", "Special", "Strategic",
    "Strong", "Tidy", "Wise",
];

/// Fruits / items for random alias generation (English)
const FRUITS_EN: &[&str] = &[
    "Apple", "Avocado", "Banana", "Blackberry", "Blueberry",
    "Broccoli", "Carrot", "Cherry", "Coconut", "Grape",
    "Lemon", "Lettuce", "Mango", "Melon", "Mushroom",
    "Onion", "Orange", "Papaya", "Peach", "Pear",
    "Pineapple", "Potato", "Pumpkin", "Raspberry", "Strawberry",
    "Tomato",
];

/// Adjectives for random alias generation (Chinese)
const ADJECTIVES_ZH: &[&str] = &[
    "迷人", "美丽", "巨大", "明亮", "干净",
    "聪明", "帅气", "可爱", "狡猾", "坚定",
    "有活力", "高效", "极好", "快速", "不错",
    "新鲜", "好", "华丽", "伟大", "英俊",
    "炽热", "善良", "诚实", "神秘", "整洁",
    "开心", "耐心", "漂亮", "强大", "富有",
    "秘密", "聪明", "稳固", "特别", "战略",
    "强大", "整齐", "智慧",
];

/// Fruits / items for random alias generation (Chinese)
const FRUITS_ZH: &[&str] = &[
    "苹果", "鳄梨", "香蕉", "黑莓", "蓝莓",
    "西兰花", "胡萝卜", "樱桃", "椰子", "葡萄",
    "柠檬", "莴苣", "芒果", "甜瓜", "蘑菇",
    "洋葱", "橙子", "木瓜", "桃子", "梨",
    "菠萝", "土豆", "南瓜", "覆盆子", "草莓",
    "番茄",
];

/// Generate a random alias in the given locale.
/// English: "{Adjective} {Fruit}" e.g. "Cute Mango"
/// Chinese: "{Adjective}的{Fruit}" e.g. "可爱的芒果"
pub fn generate_random_alias(locale: &str) -> String {
    let mut rng = rand::thread_rng();

    if locale.starts_with("zh") {
        let adj = ADJECTIVES_ZH[rng.gen_range(0..ADJECTIVES_ZH.len())];
        let fruit = FRUITS_ZH[rng.gen_range(0..FRUITS_ZH.len())];
        format!("{}的{}", adj, fruit)
    } else {
        let adj = ADJECTIVES_EN[rng.gen_range(0..ADJECTIVES_EN.len())];
        let fruit = FRUITS_EN[rng.gen_range(0..FRUITS_EN.len())];
        format!("{} {}", adj, fruit)
    }
}

/// Generate a random alias (English default)
pub fn generate_random_alias_en() -> String {
    generate_random_alias("en")
}

/// Generate a random alias (Chinese)
pub fn generate_random_alias_zh() -> String {
    generate_random_alias("zh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_en_alias() {
        let alias = generate_random_alias("en");
        assert!(!alias.is_empty());
        assert!(alias.contains(' '));
    }

    #[test]
    fn test_generate_zh_alias() {
        let alias = generate_random_alias("zh");
        assert!(!alias.is_empty());
        assert!(alias.contains('\u{7684}'));
    }

    #[test]
    fn test_uniqueness() {
        let aliases: std::collections::HashSet<String> = (0..50)
            .map(|_| generate_random_alias("en"))
            .collect();
        assert!(aliases.len() > 10);
    }
}
