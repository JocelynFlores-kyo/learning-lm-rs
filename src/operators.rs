use crate::tensor::Tensor;

// get (row) vectors from a 2D table given a list of indices 从一个二维表中根据索引列表获取行向量
pub fn gather(y: &mut Tensor<f32>, indices: &Tensor<u32>, table: &Tensor<f32>) {
    // y为输出张量，indices为索引列表，table为二维表
    let length = indices.size();    // 索引列表的长度
    let table_shape = table.shape();    // 二维表的形状
    assert!(table_shape.len() == 2);                 // 确保是二维的
    let dim = table_shape[1];                 // 二维表的列数
    assert!(y.size() == length * dim);               // 确保输出张量的大小是索引列表长度乘以二维表的列数
    for i in 0..length {                      // 遍历索引列表，获取对应的行向量
        let src = &table.data()[indices.data()[i] as usize * dim..][..dim]; // 获取二维表中的一行
        let dst = &mut unsafe { y.data_mut() }[i * dim..][..dim];       // 获取输出张量中的一行
        dst.copy_from_slice(src);
    }
}

// RoPE: Rotary Positional Embedding 实现旋转位置编码
pub fn rope(y: &mut Tensor<f32>, start_pos: usize, theta: f32) {
    let shape = y.shape();  // 获取张量的形状
    assert!(shape.len() == 3);  // 确保是三维的
    let seq_len = shape[0];    // 序列长度
    let n_heads = shape[1];   // 头数
    let d = shape[2];       // 维度
    let data = unsafe { y.data_mut() };
    for tok in 0..seq_len { 
        let pos = start_pos + tok;
        for head in 0..n_heads {
            for i in 0..d / 2 {
                let a = data[tok * n_heads * d + head * d + i];
                let b = data[tok * n_heads * d + head * d + i + d / 2];
                let freq = pos as f32 / theta.powf((i * 2) as f32 / d as f32);
                let (sin, cos) = freq.sin_cos();
                data[tok * n_heads * d + head * d + i] = a * cos - b * sin;
                data[tok * n_heads * d + head * d + i + d / 2] = b * cos + a * sin;
            }
        }
    }
}

// softmax(x) = exp(x - max) / sum(exp(x - max))
// y = softmax(mask(x)) 实现带掩码的 softmax
pub fn masked_softmax(y: &mut Tensor<f32>) {
    let ndim = y.shape().len(); // 获取张量的维度
    assert!(ndim >= 2);
    let seq_len = y.shape()[ndim - 2];  // 序列长度
    let total_seq_len = y.shape()[ndim - 1];
    let batch = y.size() / (seq_len * total_seq_len);   // 批次大小
    let data = unsafe { y.data_mut() };
    // 对每个批次的每个序列进行 softmax
    for b in 0..batch {
        let base = b * seq_len * total_seq_len;
        for i in 0..seq_len {
            let offset = base + i * total_seq_len;
            let boundary = total_seq_len - seq_len + i + 1;

            let max = data[offset..offset + boundary]
                .iter()
                .fold(data[offset], |a, b| a.max(*b));

            let sum = (0..boundary)
                .map(|j| {
                    let e = (data[offset + j] - max).exp();
                    data[offset + j] = e;
                    e
                })
                .sum::<f32>();

            (0..boundary).for_each(|j| data[offset + j] /= sum);
            (boundary..total_seq_len).for_each(|j| data[offset + j] = 0.0);
        }
    }
}

pub fn rms_norm(y: &mut Tensor<f32>, x: &Tensor<f32>, w: &Tensor<f32>, epsilon: f32) {
    let len = y.size();
    assert!(len == x.size());
    assert!(len == w.size());
    let _y = unsafe { y.data_mut() };
    let _x = x.data();
    let _w = w.data();
    let mut sum = 0.0;
    for i in 0..len {
        sum += _x[i] * _x[i];
    }
    let rms = ((sum / len as f32) + epsilon).sqrt();
    for i in 0..len {
        _y[i] = _w[i] * _x[i] / rms;
    }
    // todo!("实现 rms_norm，计算前做一些必要的检查会帮助你后续调试")
}

// y = silu(x) * y
// hint: this is an element-wise operation
pub fn swiglu(y: &mut Tensor<f32>, x: &Tensor<f32>) {
    let len = y.size();
    assert!(len == x.size());

    let _y = unsafe { y.data_mut() };
    let _x = x.data();

    for i in 0..len {
        _y[i] *= _x[i] / (1. + (-_x[i]).exp());
    }

    // todo!("实现 silu，这里给了一些前期准备工作的提示，你可以参考")
}

// C = beta * C + alpha * A @ B^T
// hint: You don't need to do an explicit transpose of B
pub fn matmul_transb(c: &mut Tensor<f32>, beta: f32, a: &Tensor<f32>, b: &Tensor<f32>, alpha: f32) {
    let c_shape = c.shape();
    let a_shape = a.shape();
    let b_shape = b.shape();
    assert!(c_shape.len() == 2);
    assert!(a_shape.len() == 2);
    assert!(b_shape.len() == 2);
    assert!(c_shape[0] == a_shape[0]);
    assert!(c_shape[1] == b_shape[0]);
    assert!(a_shape[1] == b_shape[1]);
    let m = c_shape[0];
    let n = c_shape[1];
    let k = a_shape[1];
    let _c = unsafe { c.data_mut() };
    let _a = a.data();
    let _b = b.data();
    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0;
            for l in 0..k {
                sum += _a[i * k + l] * _b[j * k + l];
            }
            _c[i * n + j] = beta * _c[i * n + j] + alpha * sum;
        }
    }
    // todo!("实现 matmul_transb，计算前做一些必要的检查会帮助你后续调试");
}

// Dot product of two tensors (treated as vectors)
#[allow(unused)]
pub fn dot(x: &Tensor<f32>, y: &Tensor<f32>) -> f32 {
    let len = x.size();
    assert!(len == y.size());
    let x_ = x.data();
    let y_ = y.data();
    let mut sum = 0.0;
    for i in 0..len {
        sum += x_[i] * y_[i];
    }
    sum
}

// Sample a index from a tensor (treated as a probability vector)
pub fn random_sample(x: &Tensor<f32>, top_p: f32, top_k: u32, temperature: f32) -> u32 {
    assert!(x.shape()[x.shape().len() - 1] == x.size());
    if temperature <= 0. || top_k < 2 || top_p <= 0. {
        return x
            .data()
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0 as _;
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct Probability {
        val: f32,
        tok: u32,
    }
    impl Eq for Probability {}
    impl PartialOrd for Probability {
        #[inline]
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Probability {
        #[inline]
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match self.val.total_cmp(&other.val) {
                std::cmp::Ordering::Equal => self.tok.cmp(&other.tok),
                ord => ord.reverse(),
            }
        }
    }
    impl From<(usize, &f32)> for Probability {
        #[inline]
        fn from((i, p): (usize, &f32)) -> Self {
            Self {
                val: p.clone(),
                tok: i as _,
            }
        }
    }

    // sort
    let mut logits = x
        .data()
        .iter()
        .enumerate()
        .map(Probability::from)
        .collect::<Vec<_>>();
    logits.sort_unstable();
    let max = core::mem::replace(&mut logits[0].val, 1.);
    // softmax & sum
    for i in 1..logits.len() {
        logits[i].val = logits[i - 1].val + ((logits[i].val - max) / temperature).exp();
    }
    // topk & topp & random
    let pk = logits[(top_k as usize).min(logits.len()) - 1].val;
    let pp = logits[logits.len() - 1].val * top_p;
    let plimit = rand::random::<f32>() * f32::min(pk, pp);
    // sample
    logits.iter().find(|p| p.val >= plimit).unwrap().tok
}

// Your implementation should at least pass the following tests:
#[test]
fn test_silu() {
    let mut y = Tensor::<f32>::new(vec![2., 3., 4.], &vec![1, 3]);
    let x = Tensor::<f32>::new(vec![1., 2., 3.], &vec![1, 3]);
    swiglu(&mut y, &x);
    assert!(y.close_to(
        &Tensor::<f32>::new(vec![1.4621172, 5.2847824, 11.43089], &vec![1, 3]),
        1e-3
    ));
}

#[test]
fn test_rms_norm() {
    let mut y = Tensor::<f32>::new(vec![1., 2., 3., 4.], &vec![2, 2]);
    let x = Tensor::<f32>::new(vec![1., 2., 3., 4.], &vec![2, 2]);
    let w = Tensor::<f32>::new(vec![1., 2.], &vec![2]);
    rms_norm(&mut y, &x, &w, 1e-6);
    assert!(y.close_to(
        &Tensor::<f32>::new(
            vec![0.6324554, 2.5298216, 0.8485281, 2.2627416],
            &vec![2, 2]
        ),
        1e-3
    ));
}

#[test]
fn test_matmul_transb() {
    let mut c = Tensor::<f32>::new(vec![1., 2., 3., 4.], &vec![2, 2]);
    let a = Tensor::<f32>::new(vec![1., 2., 3., 4., 5., 6.], &vec![2, 3]);
    let b = Tensor::<f32>::new(vec![1., 2., 3., 4., 5., 6.], &vec![2, 3]);
    matmul_transb(&mut c, 1., &a, &b, 1.);
    assert!(c.close_to(
        &Tensor::<f32>::new(vec![15., 34., 35., 81.], &vec![2, 2]),
        1e-3
    ));
}
