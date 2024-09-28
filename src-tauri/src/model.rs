use core::net;
use std::fs::File;
use std::{result, vec};

use crate::config::LlamaConfigJson;
use crate::kvcache::KVCache;
use crate::operators::{self as OP, matmul_transb, random_sample, rms_norm, silu};
use crate::params::LLamaParams;
use crate::tensor::Tensor;
use safetensors::SafeTensors;
use std::path::Path;
pub struct Llama<T> {
    vocab: usize,           // vocab size
    n_layers: usize,        // number of layers
    n_q_h: usize,           // number of heads for q
    n_kv_h: usize,          // number of heads for k and v
    d: usize,               // dimension of hidden states
    dqkv: usize,            // length of a single q, k, or v vector
    di: usize,              // dimension of intermediate states
    eps: f32,               // epsilon for RMS normalization
    rope_theta: f32,        // rope theta for rope initialization
    max_seq_len: usize,     // maximum sequence length
    params: LLamaParams<T>, // trained weights of this model
    bos_token_id: u32,      // start token id
    eos_token_id: u32,      // end token id
}

impl Llama<f32> {
    pub fn from_safetensors(model_dir: impl AsRef<Path>) -> Self {
        let config = File::open(model_dir.as_ref().join("config.json")).unwrap();
        let config: LlamaConfigJson = serde_json::from_reader(config).unwrap();
        let model_file = std::fs::read(model_dir.as_ref().join("model.safetensors")).unwrap();
        let safetensor = SafeTensors::deserialize(&model_file).unwrap();
        let params = LLamaParams::from_safetensors(&safetensor, &config);

        Self {
            vocab: config.vocab_size,
            n_layers: config.num_hidden_layers,
            n_q_h: config.num_attention_heads,
            n_kv_h: config.num_key_value_heads,
            d: config.hidden_size,
            dqkv: config.hidden_size / config.num_attention_heads,
            di: config.intermediate_size,
            eps: config.rms_norm_eps,
            rope_theta: config.rope_theta,
            max_seq_len: config.max_position_embeddings,
            params: params,
            bos_token_id: config.bos_token_id,
            eos_token_id: config.eos_token_id,
        }
    }

    pub fn new_cache(&self) -> KVCache<f32> {
        KVCache::new(self.n_layers, self.max_seq_len, self.n_kv_h * self.dqkv, 0)
    }

    pub fn forward(&self, input: &Tensor<u32>, cache: &mut KVCache<f32>) -> Tensor<f32> {
        let seq_len = input.size();
        let past_seq_len = cache.len();
        cache.increment(seq_len);
        let total_seq_len = past_seq_len + seq_len;
        let n_groups = self.n_q_h / self.n_kv_h;
        //println!("past_seq_len： {past_seq_len}");
        // Some pre-allocated buffers that will be reused
        let mut residual = Tensor::<f32>::default(&vec![seq_len, self.d]);
        let mut hidden_states = Tensor::<f32>::default(&vec![seq_len, self.d]);
        let mut q_buf = Tensor::<f32>::default(&vec![seq_len, self.n_q_h * self.dqkv]);
        let mut att_scores =
            Tensor::<f32>::default(&vec![self.n_kv_h, n_groups, seq_len, total_seq_len]);
        let mut gate_buf = Tensor::<f32>::default(&vec![seq_len, self.di]);
        let mut up_buf = Tensor::<f32>::default(&vec![seq_len, self.di]);

        // Computation Starts Here
        // Embedding lookup
        OP::gather(&mut residual, input, &self.params.embedding_table);

        for layer in 0..self.n_layers {
            OP::rms_norm(
                &mut hidden_states,
                &residual,
                &self.params.rms_att_w[layer],
                self.eps,
            );
           // let mut q_buf: Tensor<f32> = Tensor::<f32>::default(&vec![seq_len, self.n_q_h * self.dqkv]);
            let q: &mut Tensor<f32> = (&mut q_buf).reshape(&vec![seq_len, self.n_q_h * self.dqkv]); // (seq, n_h * dqkv)
            let k: &mut Tensor<f32> = &mut cache.k_cache(layer, past_seq_len); // (seq, n_kv_h * dqkv)
            let v: &mut Tensor<f32> = &mut cache.v_cache(layer, past_seq_len); // (seq, n_kv_h * dqkv)
            OP::matmul_transb(q, 0., &hidden_states, &self.params.wq[layer], 1.0);
            OP::matmul_transb(k, 0., &hidden_states, &self.params.wk[layer], 1.0);
            OP::matmul_transb(v, 0., &hidden_states, &self.params.wv[layer], 1.0);
            OP::rope(
                q.reshape(&vec![seq_len, self.n_q_h, self.dqkv]),
                past_seq_len,
                self.rope_theta,
            );
            OP::rope(
                k.reshape(&vec![seq_len, self.n_kv_h, self.dqkv]),
                past_seq_len,
                self.rope_theta,
            );

            let full_k = &mut cache.k_cache(layer, 0); // (total_seq, n_kv_h * dqkv)
            let full_v = &mut cache.v_cache(layer, 0); // (total_seq, n_kv_h * dqkv)

           // todo!("self_attention(...)");
        //    let mut att_scores =
        //     Tensor::<f32>::default(&vec![self.n_kv_h, n_groups, seq_len, total_seq_len]);
            self_attention(&mut hidden_states, &mut att_scores, q.reshape(&vec![seq_len, self.n_q_h * self.dqkv]), full_k, full_v, self.n_kv_h, n_groups, seq_len, total_seq_len, self.dqkv);
            
            OP::matmul_transb(&mut residual, 1., &hidden_states, &self.params.wo[layer], 1.0);  //输入乘以V
           
           
          // residual.print();
           // todo!("down_proj matmul and add residual");
           hidden_states = Tensor::<f32>::default(&hidden_states.shape());
          // gate_buf = Tensor::<f32>::default(&vec![seq_len, self.di]);
           //up_buf = Tensor::<f32>::default(&vec![seq_len, self.di]);
            mlp(&mut residual, &mut hidden_states, &mut gate_buf, &mut up_buf, &self.params.w_up[layer] , &self.params.w_down[layer], &self.params.w_gate[layer], &self.params.rms_ffn_w[layer], self.eps);
            //todo!("mlp(...)");
        }

        // No matter what seq_len, the output is always a 1D vector of length vocab,
        // which contains the probabilities for the next token.
        let mut logits = Tensor::<f32>::default(&vec![1, self.vocab]);
        let mut hidden_states = hidden_states.slice((seq_len - 1) * self.d, &vec![1, self.d]);
        let residual = residual.slice((seq_len - 1) * self.d, &vec![self.d]);

        OP::rms_norm(
            &mut hidden_states,
            &residual,
            &self.params.rms_out_w,
            self.eps,
        );

        OP::matmul_transb(&mut logits, 0., &hidden_states, &self.params.lm_head, 1.0);
        //logits.print();
        logits
    }

    pub fn generate(
        &self,
        token_ids: &[u32],
        max_len: usize,
        top_p: f32,
        top_k: u32,
        temperature: f32,
    ) -> Vec<u32> {
        let mut result = Vec::<u32>::from(token_ids);
        let mut cache = self.new_cache();
       let mut next = random_sample(&self.forward(&Tensor::new(result.clone(), &vec![result.len()]), &mut cache), top_p, top_k, temperature); 
       result.push(next);
        //forward(&self, input: &Tensor<u32>, cache: &mut KVCache<f32>) -> Tensor<f32> 
        while result.len() < max_len && next != self.eos_token_id  {
            let input = Tensor::new(vec![next], &vec![1]);    
            let t = self.forward(&input, &mut cache);   
          //  t.print();
            next = random_sample(&t, top_p, top_k, temperature);
           // println!("next: {next} {:?}",input.shape()); 
            result.push(next); 
          //  println!("result：{result:?}");
        }
        result
    }

    pub fn chat_generate(
        &self,
        token_ids: &[u32], 
        cache: &mut KVCache<f32>,
        max_len: usize,
        top_p: f32,
        top_k: u32,
        temperature: f32,
    ) -> Vec<u32> {
        let mut result = Vec::<u32>::from(token_ids);
       let mut next = random_sample(&self.forward(&Tensor::new(result.clone(), &vec![result.len()]), cache), top_p, top_k, temperature); 
       let mut result = Vec::<u32>::default();
       result.push(next);
            while result.len() < max_len && next != self.eos_token_id  {
                let input = Tensor::new(vec![next], &vec![1]);    
                let t = self.forward(&input, cache);   
                next = random_sample(&t, top_p, top_k, temperature);
                result.push(next); 
            }
        result

    }
}
fn self_attention(
    hidden_states: &mut Tensor<f32>, // (seq, n_kv_h * n_groups * dqkv)
    att_scores: &mut Tensor<f32>,    // (n_kv_h, n_groups, seq, total_seq) （seq, total_seq)
    q: &Tensor<f32>,                 // (seq, n_kv_h * n_groups * dqkv) (seq,  dqkv)
    k: &Tensor<f32>,                 // (total_seq, n_kv_h * dqkv) (total_seq, dqkv)
    v: &Tensor<f32>,                 // (total_seq, n_kv_h * dqkv)
    n_kv_h: usize, //多头注意力的头数
    n_groups: usize, // q的组数
    seq_len: usize,
    total_seq_len: usize,
    dqkv: usize, // embeding后词向量的大小
) {
   // todo!("Implement self_attention");
   let mut _att = unsafe { att_scores.data_mut() };
   let mut _hidden = unsafe { hidden_states.data_mut() };

   for head in 0..n_kv_h {  // 先算头数
       for group in 0..n_groups { //同一个k头会被多个 q头复用， 所以再算q的组数， 多个组复用同一个头， n_groups标识一个头被多少个组进行复用
           for seq in 0..seq_len {
            let q_start = ((seq * n_kv_h + head) * n_groups + group) * dqkv;
            let _att_start = ((head * n_groups + group) * seq_len + seq) *total_seq_len;
               for total_seq in 0..total_seq_len {
                   let mut score = 0.0;
                   let k_start = (total_seq * n_kv_h + head) * dqkv;
                   for idx in 0..dqkv {                                                              //// (total_seq, n_kv_h * dqkv) (total_seq, dqkv)
                       score  += q.data()[ q_start + idx] * k.data()[  k_start + idx ];  // k(total_seq， dqkv) 
                   }
                   _att[  _att_start + total_seq ] = score / (dqkv as f32).sqrt();
               }
           }
       }
   }
   OP::masked_softmax(att_scores);
   
   for seq in 0..seq_len {
       for head in 0..n_kv_h {
           for group in 0..n_groups {
            let _h_start = ((seq * n_kv_h + head) * n_groups + group ) * dqkv;
            let att_start = ((head * n_groups + group) * seq_len + seq) *total_seq_len;
               for idx  in 0..dqkv {
                   let mut sum = 0.0;
                   for total_seq in 0..total_seq_len {   // (n_kv_h, n_groups, seq, total_seq) （seq, total_seq)   //(total_seq, n_kv_h * dqkv)
                       sum += att_scores.data()[  att_start + total_seq] 
                            * v.data()[ (total_seq * n_kv_h  + head) * dqkv + idx];
                   }
                    // (seq, n_kv_h * n_groups * dqkv)
                   _hidden[ _h_start + idx ] = sum;
               }
           }
       }
   }

}

fn mlp(
    residual: &mut Tensor<f32>,
    hidden_states: &mut Tensor<f32>,
    gate: &mut Tensor<f32>,
    up: &mut Tensor<f32>,
    w_up: &Tensor<f32>,
    w_down: &Tensor<f32>,
    w_gate: &Tensor<f32>,
    rms_w: &Tensor<f32>,
    eps: f32,
) {
    //let mut hidden = Tensor::<f32>::default(residual.shape());
    rms_norm( hidden_states, residual, rms_w, eps);
    matmul_transb(gate, 0.0, &hidden_states, w_gate, 1.0);
    matmul_transb(up, 0.0, &hidden_states, w_up, 1.0);
    let mut tmp_up = Tensor::<f32>::default(up.shape());
    unsafe  {
        for i in 0..up.size() {
            tmp_up.data_mut()[i] = up.data()[i];
        }
    }
    silu(&mut tmp_up, &gate);
    matmul_transb(hidden_states, 0.0, &tmp_up, w_down, 1.0);
    unsafe  {
        for i in 0..residual.size() {
            residual.data_mut()[i] += hidden_states.data()[i];
        }
    }
    //silu(up, &gate);
   // matmul_transb(hi, 0.0, &hidden_states, w_up, 1.0);
    
}

#[test]
pub fn test_mlp() {
    let seq_len = 4;
    let d = 2;
    let di = 3;
    let mut residual = Tensor::<f32>::new(vec![1., 1., 1., 1., 1., 1., 1., 1.], &vec![seq_len, d]);
    let mut hidden_states = Tensor::<f32>::default(&vec![seq_len, d]);
    let mut gate_buf = Tensor::<f32>::default(&vec![seq_len, di]);
    let mut up_buf = Tensor::<f32>::default(&vec![seq_len, di]);
    let w_up = Tensor::<f32>::new(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], &vec![di, d]);
    let w_down = Tensor::<f32>::new(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], &vec![d, di]);
    let w_gate = Tensor::<f32>::new(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], &vec![di, d]);
    let rms_w = Tensor::<f32>::new(vec![1., 1.], &vec![d]);
    let eps = 1e-6;
    mlp(
        &mut residual,
        &mut hidden_states,
        &mut gate_buf,
        &mut up_buf,
        &w_up,
        &w_down,
        &w_gate,
        &rms_w,
        eps,
    );

    assert!(residual.close_to(
        &Tensor::<f32>::new(
            vec![
                1.3429964, 1.7290739, 1.3429964, 1.7290739, 1.3429964, 1.7290739, 1.3429964,
                1.7290739
            ],
            &vec![seq_len, d]
        ),
        1e-3
    ))
}

#[test]
pub fn test_load_safetensors() {
    use crate::tensor::float_eq;
    use std::path::PathBuf;
    let project_dir = env!("CARGO_MANIFEST_DIR");
    let model_dir = PathBuf::from(project_dir).join("models").join("story");
    let model = Llama::from_safetensors(model_dir);
    assert_eq!(model.vocab, 2048);
    assert_eq!(model.n_layers, 2);
    assert_eq!(model.n_q_h, 8);
    assert_eq!(model.n_kv_h, 4);
    assert_eq!(model.d, 128);
    assert_eq!(model.dqkv, 16);
    assert_eq!(model.di, 384);

    assert!(float_eq(
        &model.params.embedding_table.data()[50],
        &0.14453125,
        1e-6
    ));
    assert_eq!(
        model.params.lm_head.data()[10],
        model.params.embedding_table.data()[10]
    );
    assert!(float_eq(
        &model.params.rms_att_w[0].data()[10],
        &0.18652344,
        1e-6
    ));
    assert!(float_eq(
        &model.params.rms_ffn_w[1].data()[10],
        &0.32421875,
        1e-6
    ));
    assert!(float_eq(
        &model.params.rms_out_w.data()[100],
        &0.73046875,
        1e-6
    ));
    assert!(float_eq(
        &model.params.w_down[0].data()[100],
        &-0.0625,
        1e-6
    ));
    assert!(float_eq(&model.params.w_up[0].data()[100], &1.46875, 1e-6));
    assert!(float_eq(
        &model.params.w_gate[1].data()[100],
        &0.296875,
        1e-6
    ));
    assert!(float_eq(
        &model.params.wq[1].data()[100],
        &0.032226563,
        1e-6
    ));
    assert!(float_eq(
        &model.params.wk[1].data()[100],
        &-0.21386719,
        1e-6
    ));
    assert!(float_eq(
        &model.params.wv[0].data()[100],
        &0.041015625,
        1e-6
    ));
    assert!(float_eq(&model.params.wo[0].data()[100], &0.01965332, 1e-6));
}
