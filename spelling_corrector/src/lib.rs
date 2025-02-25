use pyo3::prelude::*;
use std::cmp::min;
use std::collections::HashMap;  
use std::collections::HashSet;
use rayon::prelude::*;


fn string_distance(a: &String, b: &String) -> u32 {
    let a = a.as_bytes();
    let b = b.as_bytes();

    let mut d: Vec<Vec<u32>> = vec![vec![0; b.len() + 1]; a.len() + 1];

    for i in 1..=a.len() {
        d[i][0] = i as u32;
    }

    for j in 1..=b.len() {
        d[0][j] = j as u32;
    }

    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost: u32 = if a[i-1] == b[j-1] { 0 } else { 1 };

            d[i][j] = min(d[i-1][j] + 1, min(d[i][j-1] + 1, d[i-1][j-1] + cost));

            // if i > 1 && j > 1 && a[i-1] == b[j-2] && a[i-2] == b[j-1] {
            //     d[i][j] = min(d[i][j], d[i-2][j-2]) + 1;
            // }
        }
    }
    return d[a.len()][b.len()]; 
}


#[pyclass(module="spelling_corrector")]
struct SpellingCorrectionSystem {
    #[pyo3(get)]
    n_grams: Vec<usize>, 

    #[pyo3(get)]
    n_grams_map: HashMap<String, HashSet<usize>>,

    #[pyo3(get)]
    word_to_index: HashMap<String, usize>, 

    #[pyo3(get)]
    index_to_word: HashMap<usize, String>,

    #[pyo3(get)]
    coocurrence_matrix: Vec<Vec<u32>>, 

    #[pyo3(get)]
    context_window: i32, 

    #[pyo3(get)]
    occurences: Vec<u32>, 
}

#[pymethods]
impl SpellingCorrectionSystem {
    #[new]
    fn new(n_grams: Vec<usize>, dictionary: Vec<String>, context_window: i32) -> Self {
        let mut instance = SpellingCorrectionSystem {
            n_grams: n_grams,
            n_grams_map: HashMap::new(),
            word_to_index: HashMap::new(), 
            index_to_word: HashMap::new(), 
            coocurrence_matrix: vec![vec![]], 
            context_window: context_window,
            occurences: vec![], 
        };    

        for word in &dictionary {
            if instance.word_to_index.contains_key(word) { continue; }
            
            let index = instance.word_to_index.len(); 
            instance.word_to_index.insert(word.clone(), index);
            instance.index_to_word.insert(index, word.clone());
        }
    
        instance.coocurrence_matrix = vec![vec![0; instance.word_to_index.len()]; instance.word_to_index.len()]; 
        instance.occurences = vec![0; instance.word_to_index.len()];

        for word_index in 0..dictionary.len() {
            for shift in -context_window..=context_window{
                let index = (word_index as i32) + shift; 
                if index >= 0 && index < (dictionary.len() as i32) {
                    let index = index as usize;

                    let word_index = *instance.word_to_index.get(&dictionary[word_index]).unwrap();
                    let context_index = *instance.word_to_index.get(&dictionary[index]).unwrap();
                    instance.coocurrence_matrix[word_index][context_index] += 1;
                    instance.coocurrence_matrix[context_index][word_index] += 1;
                }
            }
        }

        instance.__insert_words(dictionary);

        return instance; 
    }

    fn __insert_words(&mut self, words: Vec<String>) {
        for word in words {
            self.__insert_word(word);
        }
    }

    fn __insert_word(&mut self, word: String) {
        let word_index = self.word_to_index.get(&word).unwrap();
        self.occurences[*word_index] += 1;

        for n in &self.n_grams {
            if word.len() < *n {continue;}

            for i in 0..word.len() - n + 1 {
                let n_gram = &word[i..i + *n as usize];
                let entry = self.n_grams_map.entry(n_gram.to_string()).or_insert(HashSet::new());
                entry.insert(*word_index);
            }
        }
    }


    fn get_search_space(&self, word: String, n: i32, context: Vec<String>) -> Vec<(String, (u32, f32))> {
        if self.word_to_index.contains_key(&word) { return vec![(word, (0, 1.0))]; }

        let context_indices: Vec<usize> = context.into_iter().filter(|x| self.word_to_index.contains_key(x)).map(|x| *self.word_to_index.get(&x).unwrap()).collect();

        let mut search_space = HashSet::new();
        for n in &self.n_grams {
            if word.len() < *n {continue;}

            for i in 0..word.len() - n + 1 {
                let n_gram = &word[i..i + n];
                if let Some(entry) = self.n_grams_map.get(n_gram) {
                    search_space.extend(entry);
                }
            }
        }

        let mut distances: Vec<(String, (u32, f32))> = search_space.into_par_iter()
            .map(|candidate_index: usize| {
                let candidate = self.index_to_word.get(&candidate_index).unwrap().clone();
                let distance: u32 = string_distance(&candidate, &word);

                if distance < 5 {
                    let context_closeness = context_indices.iter()
                        .map(|&x| self.coocurrence_matrix[candidate_index][x]).sum::<u32>();

                    return (candidate, (distance, (context_closeness as f32)/(self.occurences[candidate_index] as f32)));
                } else {
                    return (candidate, (distance, 0.0));    
                }
            }).collect();

        distances.sort_by(|(_, b), (_, a)| u32::cmp(&b.0, &a.0));
        distances[..min(n as usize, distances.len())].to_vec()
    }

    fn get_context(&self, word: String) -> Vec<(String, u32)> {
        if !self.word_to_index.contains_key(&word) { return vec![]; }   

        let word_index = *self.word_to_index.get(&word).unwrap();
        let mut context: Vec<(usize, &u32)> = self.coocurrence_matrix[word_index].iter()
                                                                        .enumerate()
                                                                        .filter(|(index, coocurrence)| **coocurrence > 0).collect();

        context.sort_by(|a, b| a.1.cmp(&b.1));
        let context: Vec<(String, u32)> = context
                                                                    .into_par_iter()
                                                                    .map(|(index, coocurrence)| (self.index_to_word.get(&index).unwrap().clone(), *coocurrence)).collect(); 

        return context;
    }

}



#[pymodule]
fn spelling_corrector(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SpellingCorrectionSystem>()?;
    Ok(())
}
