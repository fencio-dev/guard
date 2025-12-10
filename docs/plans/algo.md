# Semantic Validation Algorithm for AI Security

# Executive Summary

A deterministic, mathematical algorithm for validating agent intent events against learned and configured design boundaries in real-time, achieving sub-10ms latency with complete explainability.

# System Architecture Overview

## Core Components

* Input: Structured hashmaps representing intent events and design boundaries  
* Encoding: Type-aware canonicalization and feature extraction  
* Projection: Sparse random projection to 128-dimensional space  
* Matching: Multi-region boundary validation using cosine similarity  
* Decision: Configurable AND/OR logic for boundary combinations  
* Learning: Continuous refinement through telemetry feedback

## Performance Targets

* Throughput: 1K-10K validations/second  
* Latency: 5-10ms per validation  
* Dimensionality: 128-dimensional embedding space  
* Explainability: Full audit trail for every decision

# Phase 1: Intent Event Processing

## 1.1 Canonicalization

**What it does:** Converts messy, inconsistent hashmaps into a clean, standardized format that always looks the same regardless of how the data was originally structured.

**The Problem it Solves:**

* The same data can be represented in different ways (nested objects, arrays, different key orders)  
* You need a consistent format so that identical intents always produce identical vectors

**How it Works:**

1. **Flattens nested structures** into dot-notation paths:  
   * `{"user": {"profile": {"age": 25}}}` becomes `"user.profile.age": 25`  
   * Arrays get indexed: `{"users": [{"name": "Alice"}]}` becomes `"users[0].name": "Alice"`  
2. **Normalizes the paths**:  
   * Makes everything lowercase  
   * Removes special characters/punctuation  
   * Sorts path components alphabetically (so order doesn't matter)  
3. **Tags each value with its type**:  
   * `"user.profile.age": (25, "int")`  
   * `"user.profile.name": ("Alice", "string")`

**Why it Matters:** Without canonicalization, the same intent event could produce different vectors just because keys were in a different order or data was structured differently. This step guarantees that semantically identical data always gets encoded the same way \- making your validation deterministic and reliable.

Transform variable-structure hashmaps into normalized representations:

| def canonicalize(hashmap):    """    Normalize keys to canonical dotted paths with type tags        Example:        {"user": {"profile": \[{"age": 25}\]}}         →         {"user.profile\[0\].age": (25, "int")}    """    canonical \= {}    for path, value in flatten\_hashmap(hashmap):        \# Normalize path tokens        tokens \= path.lower().split('.')        tokens \= \[strip\_punctuation(t) for t in tokens\]        tokens.sort()  \# Lexicographic ordering                \# Attach type tag        type\_tag \= detect\_type(value)        canonical\_path \= '.'.join(tokens)        canonical\[canonical\_path\] \= (value, type\_tag)        return canonical |
| :---- |

## 1.2 Feature Encoding

**What it does:** Converts each key-value pair from the canonicalized hashmap into a numerical vector.  
**Problem it Solves:** Machine learning algorithms can't process text/mixed data types directly \- we need numbers.

**How it Works:**

* **Keys** → vectors capturing field identity (e.g., "user.age" → 32-dim vector)  
* **Values** → type-specific encoding (numbers→standardized, strings→hashed, booleans→fixed vectors)  
* **Combines** them: `[key_vector, value_vector, key⊙value]` for richer representation

**Why it Matters:** This preserves both the "what" (field name) and "how much" (value) in a mathematical form, enabling similarity calculations between different intents while respecting data types.

### Key Encoding (e\_key)

| def encode\_key(path, d\_k=32):    """    Convert field path to vector representation        Math:        e\_key \= mean(token\_embeddings) \+ type\_embedding    """    tokens \= tokenize\_path(path)  \# e.g., "user.profile.age" → \["user", "profile", "age"\]        \# Token embeddings (lookup table or hash)    token\_vecs \= \[get\_token\_embedding(t, d\_k//2) for t in tokens\]    path\_embedding \= np.mean(token\_vecs, axis=0)        \# Type embedding    type\_embedding \= get\_type\_embedding(extract\_type(path), d\_k//2)        return np.concatenate(\[path\_embedding, type\_embedding\]) |
| :---- |

### Value Encoding (e\_val)

| def encode\_value(value, value\_type, d\_v=32):    """    Type-aware value encoding    """    if value\_type \== "numeric":        \# Standardize and squash        z \= (value \- stats\[key\]\["mean"\]) / stats\[key\]\["std"\]        return tanh(W\_numeric @ \[z, z\*\*2\])  \# W\_numeric ∈ R^{d\_v × 2}        elif value\_type \== "string":        if is\_categorical(value):            return categorical\_embedding\[value\]  \# Lookup table        else:            \# Character n-gram hashing            ngrams \= extract\_ngrams(value, n=3)            sparse\_vec \= hash\_ngrams(ngrams, dim=1024)            return W\_string @ sparse\_vec  \# W\_string ∈ R^{d\_v × 1024}        elif value\_type \== "boolean":        return bool\_embeddings\[value\]  \# Fixed vectors        elif value\_type \== "array":        element\_vecs \= \[encode\_value(elem, detect\_type(elem)) for elem in value\]        return np.concatenate(\[            np.mean(element\_vecs, axis=0),            np.max(element\_vecs, axis=0),            encode\_numeric(len(value))        \])        elif value\_type \== "nested\_map":        return encode\_hashmap(value)  \# Recursive call |
| :---- |

### Combined Field Features

**Element-wise Product (⊙) in Feature Combination**  
The element-wise product `e_key ⊙ e_val` captures **interaction effects** between the field identity and its value.

**What it achieves:**

* Creates features that represent "this specific field having this specific value"  
* Example: The pattern "user.age=high" gets a unique signature different from "user.salary=high" even though both have "high" values  
* Enables the model to learn that certain values are significant only in certain contexts (e.g., a value of "100" means something different for "temperature\_celsius" vs "user\_count")

**Why not just concatenate?** Simple concatenation `[e_key, e_val]` treats field identity and value as independent. The product term adds **non-linear interactions** that help distinguish subtle patterns like "this field being empty" vs "that field being empty" \- critical for security boundaries where context determines whether something is suspicious.

| def create\_field\_vector(e\_key, e\_val, d\_field=96):    """    Combine key and value encodings        Math:        f \= \[e\_key; e\_val; e\_key ⊙ e\_val\]    """    interaction \= e\_key \* e\_val  \# Element-wise product    return np.concatenate(\[e\_key, e\_val, interaction\]) |
| :---- |

## 1.3 Permutation-Invariant Aggregation

**Purpose:** Combines all field vectors into one fixed-size vector that represents the entire hashmap, regardless of field order.

**The Problem:** Hashmaps have variable numbers of fields and no inherent ordering. The same data with fields in different order must produce the same final vector.

**How it Works:** Takes all field vectors `{f₁, f₂, ..., fₙ}` and applies order-independent operations:

* **Mean**: Captures average behavior across all fields  
* **Max**: Captures extreme/peak values in any field  
* **Sum**: Captures overall magnitude/presence of features

Concatenates these: `z = [mean, max, sum]`

**Why These Operations?** They're mathematically **commutative** \- order doesn't affect the result. This guarantees that `{"a": 1, "b": 2}` and `{"b": 2, "a": 1}` produce identical vectors, making your validation deterministic regardless of how the hashmap was constructed or traversed.

| def aggregate\_field\_vectors(field\_vectors):    """    Pool field vectors into fixed-size representation        Math:        z \= \[mean({f\_i}); max({f\_i}); sum({f\_i})\]    """    field\_array \= np.array(field\_vectors)        z\_mean \= np.mean(field\_array, axis=0)    z\_max \= np.max(field\_array, axis=0)    z\_sum \= np.sum(field\_array, axis=0)        return np.concatenate(\[z\_mean, z\_max, z\_sum\])  \# 3 × d\_field dimensions |
| :---- |

## 1.4 Sparse Random Projection

**Purpose:** Compresses the high-dimensional aggregated vector (potentially 288+ dims) down to exactly 128 dimensions while preserving distances between vectors.

**The Problem:** After aggregation, vectors are too high-dimensional for efficient computation and storage, but we can't lose the ability to measure similarity accurately.

**How it Works:** Multiplies by a sparse random matrix where each element is:

* `+√3` with probability 1/6  
* `0` with probability 2/3  
* `-√3` with probability 1/6

Then normalizes: `z_norm = (R × z) / ||R × z||`

**Why Sparse?**

* **2/3 zeros** \= 3x faster multiplication than dense matrices  
* **Mathematically proven** (Johnson-Lindenstrauss) to preserve distances within ε distortion  
* **Deterministic** once matrix R is generated (use fixed seed)  
* **No training needed** \- works immediately, perfect for cold start

The normalization at the end ensures all vectors lie on the unit sphere, making cosine similarity just a simple dot product.

| def create\_sparse\_projection\_matrix(d\_in, d\_out=128, sparsity=0.66):    """    Create sparse random projection matrix        Math:        R\[i,j\] \= √(s) × {+1 with prob 1/(2s),                          0 with prob 1-1/s,                         \-1 with prob 1/(2s)}        where s \= 1/sparsity    """    s \= 1 / sparsity    sqrt\_s \= np.sqrt(s)        R \= np.random.choice(        \[sqrt\_s, 0, \-sqrt\_s\],        size=(d\_out, d\_in),        p=\[1/(2\*s), 1-1/s, 1/(2\*s)\]    )    return Rdef project\_to\_embedding\_space(z\_featurized, R\_sparse):    """    Project and normalize        Math:        z\_128 \= R × z\_featurized        z\_norm \= z\_128 / ||z\_128||₂    """    z\_128 \= R\_sparse @ z\_featurized    z\_norm \= z\_128 / np.linalg.norm(z\_128)    return z\_norm |
| :---- |

# Phase 2: Boundary Matching

This phase takes the normalized 128-dim intent vector and efficiently determines which design boundaries it falls within, using a two-stage filtering approach to handle potentially thousands of boundaries in milliseconds.

## 2.1 Inverted Index Lookup

*Note – Let’s ignore this implementation for now because this needs deterministic attribute extraction as well. If we see that the operations are taking out of budget amounts of time to perform, we will introduce more optimisation techniques like these.*

**Purpose:** Quickly filters thousands of boundaries down to only the relevant ones (typically 10-100) before doing expensive vector comparisons.

**The Problem:** Checking every boundary for every intent would be O(n) \- too slow when you have thousands of boundaries.

**How it Works:**

* **Build time**: Extract high-level attributes from each boundary (domain="finance", action="read", resource="database")  
* **Query time**: Extract same attributes from intent, lookup which boundaries have matching attributes  
* Returns only boundaries that could possibly apply to this intent

**Why it Matters:** This eliminates \~90% of boundaries before any math happens. Example: An intent with `action="read"` never needs to check boundaries that only apply to `action="delete"`. This structural pre-filtering is deterministic, explainable, and reduces your vector comparisons from thousands to dozens, keeping latency under 10ms.

| class InvertedIndex:    """    Fast boundary filtering by high-level attributes    """    def \_\_init\_\_(self):        self.index \= defaultdict(set)  \# attribute → boundary\_ids        def build(self, boundaries):        for boundary\_id, boundary in boundaries.items():            for attr in extract\_attributes(boundary):                self.index\[attr\].add(boundary\_id)        def query(self, intent\_attributes):        candidate\_ids \= set()        for attr in intent\_attributes:            candidate\_ids.update(self.index.get(attr, set()))        return candidate\_ids |
| :---- |

## 2.2 LSH for Cosine Similarity

**Purpose:** Quickly finds which prototype/anchor vectors are similar to the intent vector without checking every single one.

**The Problem:** Even after inverted indexing, each boundary might have dozens of regions with prototypes/anchors. Checking all of them is still expensive.

**How it Works:**

* **Setup**: Create random hyperplanes in 128-dim space, use them to generate binary hash codes  
* **Indexing**: Each prototype/anchor gets hashed to buckets like "01101..." based on which side of each hyperplane it falls  
* **Query**: Hash the intent vector, only check vectors in the same or nearby buckets  
* Similar vectors have high probability of colliding in the same bucket

**Math Insight:** If two vectors have cosine similarity 0.9, they'll hash to the same bucket \~80% of the time. If similarity is 0.3, collision probability drops to \~20%.

**Why it Matters:** Reduces similarity checks from O(n) to O(1) average case. You check maybe 50 candidates instead of 1000+, while still finding the truly similar ones with high probability. This is what makes sub-10ms latency achievable even with complex boundaries.

| class CosineLSH:    """    Locality-Sensitive Hashing optimized for cosine similarity        Math:        Hash function: h(x) \= sign(r · x) where r \~ N(0, I)        Probability of collision: P\[h(x) \= h(y)\] \= 1 \- θ(x,y)/π    """    def \_\_init\_\_(self, d=128, num\_tables=10, hash\_size=16):        self.tables \= \[\]        for \_ in range(num\_tables):            \# Random hyperplanes for this table            R \= np.random.randn(hash\_size, d)            R \= R / np.linalg.norm(R, axis=1, keepdims=True)            self.tables.append({                'hyperplanes': R,                'buckets': defaultdict(list)            })        def hash\_vector(self, v, table\_idx):        """Generate binary hash for vector v"""        R \= self.tables\[table\_idx\]\['hyperplanes'\]        projections \= R @ v        hash\_code \= ''.join(\['1' if p \>= 0 else '0' for p in projections\])        return hash\_code        def insert(self, vector\_id, vector):        """Add vector to all hash tables"""        for i, table in enumerate(self.tables):            hash\_code \= self.hash\_vector(vector, i)            table\['buckets'\]\[hash\_code\].append((vector\_id, vector))        def query(self, query\_vector, max\_candidates=50):        """Find approximate nearest neighbors"""        candidates \= set()        for i, table in enumerate(self.tables):            hash\_code \= self.hash\_vector(query\_vector, i)            bucket \= table\['buckets'\].get(hash\_code, \[\])            candidates.update(bucket)            if len(candidates) \>= max\_candidates:                break        return list(candidates)\[:max\_candidates\] |
| :---- |

## 2.3 Distance Computation

**Purpose:** Calculates the actual cosine similarity between the intent vector and candidate prototypes/anchors retrieved from LSH.

**The Problem:** LSH gives us likely candidates, but we need exact similarity scores to make allow/block decisions.

**How it Works:**

* For each boundary's candidate regions from LSH:  
  * Compute dot product: `similarity = z_norm · prototype` (since vectors are normalized)  
  * Track the maximum similarity across all regions in that boundary  
  * Record which prototype/anchor was closest (for explainability)

**Key Optimization:** Since all vectors are pre-normalized to unit length, cosine similarity reduces to a simple dot product \- no division needed. This makes batch computation extremely fast using SIMD operations.

**Why it Matters:** This gives you the exact similarity scores needed for threshold comparison. The max similarity per boundary determines if the intent is "inside" (similarity ≥ threshold) or "outside" (similarity \< threshold). Recording the closest match also provides explainability: "Blocked because similarity to compliance.data\_access prototype was 0.72, needed 0.85".

| def compute\_boundary\_distances(z\_norm, boundaries, lsh\_index):    """    Compute cosine similarity to all regions in relevant boundaries        Math:        cosine\_sim(x, y) \= x · y / (||x|| × ||y||)        Since vectors are normalized: cosine\_sim(x, y) \= x · y    """    boundary\_scores \= {}        for boundary\_id in boundaries:        \# Get candidate regions from LSH        candidate\_regions \= lsh\_index.query(z\_norm)                max\_similarity \= \-1        best\_region \= None                for region\_id, region\_vector in candidate\_regions:            if region\_id.startswith(boundary\_id):                \# Compute cosine similarity (dot product of normalized vectors)                similarity \= np.dot(z\_norm, region\_vector)                                if similarity \> max\_similarity:                    max\_similarity \= similarity                    best\_region \= region\_id                boundary\_scores\[boundary\_id\] \= {            'similarity': max\_similarity,            'closest\_region': best\_region        }        return boundary\_scores |
| :---- |

## 2.4 Multi-Region Boundary Representation

**Purpose:** Represents each design boundary as multiple disconnected "safe zones" in the 128-dim space, rather than a single region.

**The Problem:** Real-world policies often have multiple valid contexts. Example: "Allow database access FROM analytics team OR admin role OR during maintenance window" \- these are completely different patterns that can't be captured by a single sphere/region.

**How it Works:** Each boundary contains:

* **Multiple regions**, each with a prototype (center point) and optional anchors (validated edge cases)  
* **Single threshold** that applies to all regions  
* **Type flag**: mandatory (must satisfy) or optional (nice to have)  
* Evaluation finds the closest region and checks if similarity ≥ threshold

**Key Design:**

* **Prototypes**: Learned centroids representing typical valid patterns  
* **Anchors**: Real validated intents added over time to expand coverage  
* Intent is "within boundary" if it's close to ANY region

**Why it Matters:** This flexibility lets you model complex, real-world policies without forcing them into oversimplified single-region constraints. It also enables organic growth \- start with one prototype, add anchors as you observe valid edge cases.

| class Boundary:    """    A boundary with multiple disconnected safe regions    """    def \_\_init\_\_(self, boundary\_id, boundary\_type="mandatory",  threshold=0.85):        self.id \= boundary\_id        self.type \= boundary\_type  \# "mandatory" or "optional"        self.threshold \= threshold        self.regions \= \[\]  \# List of (prototype, anchors) tuples        def add\_region(self, prototype, anchors=None):        """Add a new safe region"""        self.regions.append({            'prototype': prototype / np.linalg.norm(prototype),            'anchors': anchors if anchors else \[\],            'created\_at': time.time()        })        def evaluate(self, z\_norm):        """Check if vector is within any region"""        max\_sim \= \-1        best\_match \= None                for region in self.regions:            \# Check prototype            sim \= np.dot(z\_norm, region\['prototype'\])            if sim \> max\_sim:                max\_sim \= sim                best\_match \= ('prototype', region)                        \# Check anchors            for anchor in region\['anchors'\]:                sim \= np.dot(z\_norm, anchor)                if sim \> max\_sim:                    max\_sim \= sim                    best\_match \= ('anchor', anchor)                return {            'within\_boundary': max\_sim \>= self.threshold,            'similarity': max\_sim,            'match\_type': best\_match\[0\] if best\_match else None,            'gap': self.threshold \- max\_sim if max\_sim \< self.threshold else 0        } |
| :---- |

# Phase 3: Decision Aggregation

This phase combines individual boundary evaluation results into a final ALLOW/BLOCK decision using configurable logic, then generates detailed explanations for audit trails.

## 3.1 Policy Combination Logic

**Purpose:** Combines multiple boundary decisions based on their types (mandatory vs optional) into a single verdict.

**The Problem:** Different boundaries have different importance \- some are hard requirements (compliance), others are suggestions (best practices).

**How it Works:**

* **Mandatory boundaries**: ALL must pass (strict AND logic) \- single failure \= BLOCK  
* **Optional boundaries**: Weighted voting \- calculate `weighted_score = Σ(similarity × weight) / Σ(weights)`  
* Final decision: ALLOW only if all mandatory pass AND optional score ≥ threshold

**Why it Matters:** This mimics real organizational policies where some rules are non-negotiable (regulatory compliance) while others are flexible (internal guidelines). The weighted scoring for optional boundaries lets you express "satisfy at least 2 out of 3 soft requirements" type policies.

| def aggregate\_boundary\_decisions(boundary\_evaluations, optional\_weight\_threshold=0.5):    """    Combine multiple boundary decisions based on their types        Logic:        \- ALL mandatory boundaries must be satisfied (AND)        \- Weighted threshold for optional boundaries (OR)    """    mandatory\_results \= \[\]    optional\_results \= \[\]        for boundary\_id, evaluation in boundary\_evaluations.items():        if evaluation\['boundary'\].type \== 'mandatory':            mandatory\_results.append(evaluation)        else:            optional\_results.append(evaluation)        \# Check mandatory boundaries    mandatory\_satisfied \= all(r\['within\_boundary'\] for r in mandatory\_results)    mandatory\_violations \= \[r for r in mandatory\_results if not r\['within\_boundary'\]\]        if not mandatory\_satisfied:        return {            'decision': 'BLOCK',            'reason': 'mandatory\_boundary\_violation',            'violations': mandatory\_violations,            'confidence': 1.0        }        \# Check optional boundaries (if mandatory passed)    if optional\_results:        weights \= \[r.get('weight', 1.0) for r in optional\_results\]        scores \= \[r\['similarity'\] \* w for r, w in zip(optional\_results, weights)\]        weighted\_score \= sum(scores) / sum(weights)                optional\_satisfied \= weighted\_score \>= optional\_weight\_threshold    else:        optional\_satisfied \= True        weighted\_score \= 1.0        return {        'decision': 'ALLOW' if optional\_satisfied else 'BLOCK',        'reason': 'passed\_all\_checks' if optional\_satisfied else 'optional\_threshold\_not\_met',        'mandatory\_score': min(\[r\['similarity'\] for r in mandatory\_results\]) if mandatory\_results else 1.0,        'optional\_score': weighted\_score,        'confidence': weighted\_score if optional\_satisfied else 1.0 \- weighted\_score    } |
| :---- |

## 3.2 Explainability Generation

**Purpose:** Creates a detailed, auditable record of exactly why an intent was allowed or blocked.

**The Problem:** Security teams need to understand and debug decisions, especially for false positives/negatives.

**How it Works:** Records for each boundary:

* Similarity score vs threshold (e.g., "0.72 \< 0.85")  
* Which region/prototype matched  
* Violation severity and gap to threshold  
* Sorted by importance (violations first, mandatory before optional)

**Output Example:** "Blocked: Violated mandatory boundary 'compliance.data\_access' (similarity=0.72, required=0.85, gap=0.13)"

**Why it Matters:** Complete forensics for every decision enables debugging, compliance audits, and threshold tuning. The structured format also feeds directly into the learning pipeline to identify patterns in false positives/negatives.

| def generate\_explanation(intent\_vector, boundary\_evaluations, decision):    """    Create detailed audit trail for the decision    """    explanation \= {        'timestamp': time.time(),        'decision': decision\['decision'\],        'reason': decision\['reason'\],        'confidence': decision\['confidence'\],        'boundary\_details': \[\]    }        for boundary\_id, eval\_result in boundary\_evaluations.items():        detail \= {            'boundary\_id': boundary\_id,            'boundary\_type': eval\_result\['boundary'\].type,            'threshold': eval\_result\['boundary'\].threshold,            'similarity': eval\_result\['similarity'\],            'within\_boundary': eval\_result\['within\_boundary'\],            'gap': eval\_result.get('gap', 0),            'closest\_region': eval\_result.get('match\_type')        }                if not eval\_result\['within\_boundary'\]:            detail\['violation\_severity'\] \= 'high' if eval\_result\['boundary'\].type \== 'mandatory' else 'low'            detail\['recommendation'\] \= f"Intent needs {eval\_result\['gap'\]:.3f} more similarity to pass"                explanation\['boundary\_details'\].append(detail)        \# Sort by importance (violations first, then by type)    explanation\['boundary\_details'\].sort(        key=lambda x: (not x\['within\_boundary'\], x\['boundary\_type'\] \== 'mandatory'),        reverse=True    )        return explanation |
| :---- |

# Phase 4: Boundary Learning & Refinement

This phase continuously improves boundaries by collecting telemetry from production decisions and using feedback to add anchors, adjust thresholds, and discover new regions.

## 4.1 Telemetry Collection

**Purpose:** Captures every decision and its outcome for analysis and learning.

**The Problem:** Without feedback, boundaries remain static and can't adapt to new valid patterns or eliminate false positives.

**How it Works:**

* Logs every decision: intent vector, verdict, similarity scores, timestamp  
* Collects feedback labels: true/false positive/negative (from human review or downstream signals)  
* Maintains circular buffer (e.g., last 10K decisions) and feedback queue  
* Links decisions to outcomes via unique IDs

**Why it Matters:** This creates the labeled dataset needed for boundary evolution. False negatives reveal valid patterns you're blocking (need expansion), while false positives show where boundaries are too permissive (need tightening). The buffer provides immediate context while the queue enables async feedback integration.

| class TelemetryCollector:    """    Collect and store decision data for learning    """    def \_\_init\_\_(self, buffer\_size=10000):        self.buffer \= deque(maxlen=buffer\_size)        self.feedback\_queue \= Queue()        def log\_decision(self, intent\_vector, decision, explanation):        """Log every decision for analysis"""        entry \= {            'id': uuid.uuid4().hex,            'timestamp': time.time(),            'intent\_vector': intent\_vector.tolist(),            'decision': decision,            'explanation': explanation,            'feedback': None  \# Filled in later        }        self.buffer.append(entry)        return entry\['id'\]        def add\_feedback(self, decision\_id, feedback\_type, metadata=None):        """        Record feedback on decisions                feedback\_type: 'true\_positive', 'true\_negative', 'false\_positive', 'false\_negative'        """        self.feedback\_queue.put({            'decision\_id': decision\_id,            'feedback\_type': feedback\_type,            'metadata': metadata,            'timestamp': time.time()        }) |
| :---- |

## 4.2 Boundary Evolution

**Purpose:** Updates boundaries based on accumulated feedback to reduce false positives/negatives.

**The Problem:** Initial boundaries are imperfect \- too strict in some areas, too loose in others. They need to adapt based on real-world usage.

**How it Works:** Three evolution mechanisms:

1. **Anchor Addition**: False negatives with similarity \>0.7 become anchors (expands coverage)  
2. **Threshold Tuning**: Analyzes score distributions to find optimal cutoff  
3. **Region Splitting**: High-variance regions (\>50 anchors) split via k-means

**Update Criteria:**

* Minimum 100 samples before making changes  
* Updates run offline (hourly/daily batches)  
* New regions require human approval

**Why it Matters:** Boundaries organically grow to cover legitimate edge cases while maintaining security. The 0.7 similarity threshold for anchors ensures you only expand for "near misses," not completely novel patterns. This achieves continuous improvement without model retraining.

| class BoundaryEvolver:    """    Evolve boundaries based on telemetry feedback    """    def \_\_init\_\_(self, min\_samples\_for\_update=100):        self.min\_samples \= min\_samples\_for\_update        def evolve\_boundary(self, boundary, telemetry\_data):        """        Update boundary based on feedback        """        false\_negatives \= \[t for t in telemetry\_data                           if t\['feedback'\] \== 'false\_negative'                           and boundary.id in t\['explanation'\]\['boundary\_details'\]\]                if len(false\_negatives) \< self.min\_samples:            return boundary  \# Not enough data                updates \= \[\]                \# 1\. Add anchors for near-misses        for fn in false\_negatives:            eval\_detail \= next(b for b in fn\['explanation'\]\['boundary\_details'\]                              if b\['boundary\_id'\] \== boundary.id)                        if eval\_detail\['similarity'\] \> 0.7:  \# Near-miss threshold                \# Add as anchor to closest region                vector \= np.array(fn\['intent\_vector'\])                closest\_region \= self.\_find\_closest\_region(vector, boundary)                updates.append(('add\_anchor', closest\_region, vector))                \# 2\. Adjust thresholds based on score distributions        all\_scores \= self.\_extract\_scores(telemetry\_data, boundary.id)        new\_threshold \= self.\_optimize\_threshold(all\_scores)        if abs(new\_threshold \- boundary.threshold) \> 0.02:            updates.append(('adjust\_threshold', new\_threshold))                \# 3\. Split high-variance regions        for region\_idx, region in enumerate(boundary.regions):            if len(region\['anchors'\]) \> 50:                variance \= self.\_compute\_region\_variance(region)                if variance \> 0.3:  \# High variance threshold                    new\_regions \= self.\_split\_region(region)                    updates.append(('split\_region', region\_idx, new\_regions))                return self.\_apply\_updates(boundary, updates)        def \_optimize\_threshold(self, scores):        """        Find optimal threshold using ROC analysis                Math:            threshold\* \= argmax\_t (TPR(t) \- FPR(t))        """        true\_positive\_scores \= scores\['true\_positive'\] \+ scores\['false\_negative'\]        true\_negative\_scores \= scores\['true\_negative'\] \+ scores\['false\_positive'\]                thresholds \= np.linspace(0.5, 1.0, 50)        best\_threshold \= None        best\_score \= \-1                for t in thresholds:            tpr \= np.mean(true\_positive\_scores \>= t)  \# True positive rate            fpr \= np.mean(true\_negative\_scores \>= t)  \# False positive rate            score \= tpr \- fpr                        if score \> best\_score:                best\_score \= score                best\_threshold \= t                return best\_threshold |
| :---- |

## 4.3 Projection Matrix Learning (Future)

**Purpose:** Replaces random sparse projection with learned projection optimized for your specific boundary patterns.

**The Problem:** Random projections preserve all distances equally, but some dimensions might be more important for distinguishing allowed vs blocked intents.

**How it Works:**

* Uses accumulated labeled data (allow/block decisions with feedback)  
* Learns projection matrix via metric learning (LDA, contrastive learning, etc.)  
* Objective: Maximize separation between allowed and blocked intent clusters  
* Validates on held-out data before deployment

**Implementation:** Start with Linear Discriminant Analysis for simplicity, evolve to neural metric learning if needed.

**Why it Matters:** Learned projections can improve accuracy by 20-30% by focusing on the dimensions that matter for your specific security policies. This is your "v2" upgrade path once you have sufficient production data \- keeping the same algorithm structure but with better vector representations.

| def learn\_projection\_matrix(labeled\_data, d\_in, d\_out=128, margin=0.1):    """    Learn projection matrix to maximize separation between allowed/blocked        Math:        Objective: max ∑\_i,j y\_ij × (margin \- ||W×x\_i \- W×x\_j||₂)        where y\_ij \= \+1 if same class, \-1 if different class                Can be solved via:        \- Metric learning (LMNN, NCA)        \- Contrastive learning        \- Linear discriminant analysis    """    X\_allow \= np.array(\[d\['vector'\] for d in labeled\_data if d\['label'\] \== 'allow'\])    X\_block \= np.array(\[d\['vector'\] for d in labeled\_data if d\['label'\] \== 'block'\])        \# Simple LDA-based approach for initialization    from sklearn.discriminant\_analysis import LinearDiscriminantAnalysis        lda \= LinearDiscriminantAnalysis(n\_components=min(d\_out, len(np.unique(labels))\-1))    X\_all \= np.vstack(\[X\_allow, X\_block\])    y\_all \= np.hstack(\[np.ones(len(X\_allow)), np.zeros(len(X\_block))\])        lda.fit(X\_all, y\_all)    W\_learned \= lda.coef\_\[:d\_out\]        \# Pad with random projections if needed    if W\_learned.shape\[0\] \< d\_out:        W\_random \= create\_sparse\_projection\_matrix(d\_in, d\_out \- W\_learned.shape\[0\])        W\_learned \= np.vstack(\[W\_learned, W\_random\])        \# Normalize rows    W\_learned \= W\_learned / np.linalg.norm(W\_learned, axis=1, keepdims=True)        return W\_learned |
| :---- |

# Mathematical Foundations

## Distance Preservation (Johnson-Lindenstrauss Lemma)

The sparse random projection provably preserves distances within (1±ε) distortion. For 1000 points with 10% distortion tolerance, you need only \~100 dimensions (not 1000s). This guarantees that similar intents stay similar and different intents stay different after projection, making 128 dimensions more than sufficient for most use cases.

For sparse random projections:

| With high probability, for any vectors u, v:(1-ε)||u-v||² ≤ ||Ru \- Rv||² ≤ (1+ε)||u-v||²Required dimensions: d\_out ≥ O(log(n)/ε²)where n \= number of points, ε \= distortion tolerance |
| :---- |

## Cosine Similarity Properties

Measures angular similarity between vectors, ranging from \-1 (opposite) to 1 (identical). For normalized vectors, it's just a dot product. Key thresholds: 0.87 \= very similar (30°), 0.71 \= similar (45°), 0.50 \= somewhat similar (60°). This geometric interpretation helps set intuitive thresholds for boundaries.

| cos(θ) \= (x·y)/(||x||×||y||)For normalized vectors:\- cos(θ) ∈ \[-1, 1\]\- cos(θ) \= 1 → identical direction\- cos(θ) \= 0 → orthogonal\- cos(θ) \= \-1 → opposite directionSimilarity threshold mapping:\- θ \= 0° → cos(θ) \= 1.00 (identical)\- θ \= 30° → cos(θ) \= 0.87 (very similar)\- θ \= 45° → cos(θ) \= 0.71 (similar)\- θ \= 60° → cos(θ) \= 0.50 (somewhat similar)\- θ \= 90° → cos(θ) \= 0.00 (unrelated) |
| :---- |

## LSH Collision Probability

Random hyperplanes partition space so similar vectors likely fall in the same partition. Collision probability \= 1 \- θ/π where θ is angle between vectors. Multiple hash functions (AND) increase precision; multiple tables (OR) increase recall. With 10 tables of 16 bits each, you get excellent recall for similarities \>0.8.

For cosine similarity with random hyperplane hashing:

| P\[h(x) \= h(y)\] \= 1 \- θ(x,y)/πwhere θ(x,y) \= arccos(x·y/(||x||×||y||))Multiple hash functions (AND):P\_AND \= (1 \- θ/π)^kMultiple tables (OR):P\_OR \= 1 \- (1 \- P\_AND)^L |
| :---- |

## Permutation Invariance

Using commutative operations (sum, mean, max) ensures the same hashmap always produces the same vector regardless of field ordering. This is critical for deterministic validation \- `{"a":1, "b":2}` and `{"b":2, "a":1}` must map to identical vectors. Without this property, the system would be unreliable and non-reproducible.

For any permutation σ of field indices:

| Aggregate({f\_1, f\_2, ..., f\_n}) \= Aggregate({f\_σ(1), f\_σ(2), ..., f\_σ(n)})Achieved through:\- Commutative operations (sum, mean, max)\- Order-independent feature extraction |
| :---- |

# Implementation Optimization

## 1\. Vectorization

| \# Instead of loops, use numpy broadcastingsimilarities \= intent\_matrix @ boundary\_matrix.T  \# Batch computation |
| :---- |

## 2\. Caching

| @lru\_cache(maxsize=10000)def encode\_and\_project(canonical\_hashmap\_str):    """Cache frequently seen intents"""    return project\_to\_embedding\_space(encode\_hashmap(canonical\_hashmap\_str)) |
| :---- |

## 3\. SIMD Operations

| \# Use numpy's BLAS-optimized operationsnp.dot(a, b)  \# Automatically uses SIMD/AVX when available |
| :---- |

## 4\. Sparse Matrix Operations

| from scipy.sparse import csr\_matrix\# Use sparse matrices for projectionR\_sparse \= csr\_matrix(R)projected \= R\_sparse.dot(feature\_vector) |
| :---- |

## 5\. Parallel Processing

| from concurrent.futures import ThreadPoolExecutordef batch\_validate(intent\_vectors, boundaries):    with ThreadPoolExecutor(max\_workers=4) as executor:        results \= executor.map(            lambda v: validate\_intent(v, boundaries),            intent\_vectors        )    return list(results) |
| :---- |

# Deployment Considerations

## Python → Rust Bridge

| \# Use PyO3 or rust-cpython for critical pathsimport rust\_validator  \# Rust extension moduledef validate\_intent\_hybrid(intent, boundaries):    \# Heavy computation in Rust    distances \= rust\_validator.compute\_distances(intent, boundaries)        \# Decision logic in Python (easier to modify)    return aggregate\_boundary\_decisions(distances) |
| :---- |

## Memory Layout

| \# Use contiguous memory for cache efficiencyvectors \= np.ascontiguousarray(vectors, dtype=np.float32) |
| :---- |

## Monitoring Metrics

| metrics\_to\_track \= {    'p50\_latency': histogram('validation\_latency', percentile=50),    'p99\_latency': histogram('validation\_latency', percentile=99),    'throughput': rate('validations\_per\_second'),    'cache\_hit\_rate': ratio('cache\_hits', 'total\_requests'),    'false\_positive\_rate': ratio('false\_positives', 'total\_allows'),    'false\_negative\_rate': ratio('false\_negatives', 'total\_blocks'),    'boundary\_coverage': unique\_count('boundaries\_matched'),} |
| :---- |

# Appendix: Configuration Schema

| semantic\_sandbox:  \# Encoding parameters  encoding:    key\_dim: 32    value\_dim: 32    field\_dim: 96      \# Projection parameters  projection:    output\_dim: 128    sparsity: 0.66    random\_seed: 42      \# LSH parameters  lsh:    num\_tables: 10    hash\_size: 16    max\_candidates: 50      \# Boundary parameters  boundaries:    default\_threshold: 0.85    near\_miss\_threshold: 0.70    optional\_weight\_threshold: 0.50      \# Learning parameters  learning:    min\_samples\_for\_update: 100    anchor\_addition\_threshold: 0.70    region\_split\_variance: 0.30    threshold\_optimization\_range: \[0.5, 1.0\]      \# Performance parameters  performance:    max\_latency\_ms: 10    cache\_size: 10000    batch\_size: 100    num\_workers: 4 |
| :---- |

# Comparing Design Boundaries and Intent Events Meaningfully

Both **design boundaries** and **intent events** are structurally different data objects, but the system compares them in a shared 128-dimensional embedding space. To ensure these comparisons remain semantically valid, the embedding vector is divided into fixed **slices**—dedicated dimension ranges representing specific conceptual categories.

## Slice-Based Representation

Each embedding vector is partitioned as follows:

* **Action slice (0–31):** Represents the type of operation being performed (read, write, delete, etc.).  
* **Resource slice (32–63):** Captures the resource path or entity on which the action operates.  
* **Data slice (64–95):** Encodes the sensitivity or classification of the information being accessed.  
* **Risk slice (96–127):** Encodes contextual or behavioral risk factors such as authentication state or request origin.

By enforcing a consistent slot structure, both boundary vectors and intent vectors remain aligned in meaning. Cosine similarity is computed **per slice**, not across the whole vector. This prevents unrelated semantics—like resource information influencing action similarity—from distorting comparisons.

## Hybrid Evaluation

* **Exact checks:** Boolean and categorical attributes (authentication required, time window, deny list) are evaluated directly before any embedding comparison.  
* **Per-slice similarity:** Each slice’s cosine similarity is compared to its own threshold.  
  * Mandatory boundaries use the minimum per-slice similarity.  
  * Optional boundaries use a weighted average across slices.  
* **Aggregation:** Final decisions combine slice scores according to boundary type and severity.

## Benefits

* **Interpretability:** Each slice provides explainable similarity contributions.  
* **Stability:** Encoding drift in one category does not contaminate others.  
* **Performance:** Slice-wise cosine comparisons remain lightweight and parallelizable.  
* **Calibration:** Thresholds and weights can be tuned independently per slice and per boundary.

This structured approach ensures that even though design boundaries and intent events have different schemas, their vector representations remain aligned enough for deterministic, interpretable, and latency-safe semantic matching.

# Versioned Schema and Slot Contracts

**Purpose.** Ensure boundary vectors and intent vectors are comparable.

**Spec.**

* Define a versioned schema with required slots: `action`, `resource`, `data`, `risk`.  
* For each slot, **publish vocabularies, units, and scaling rules**.  
* Reserve fixed embedding slices: action 0–31, resource 32–63, data 64–95, risk 96–127.  
* Require per-slice thresholds in every boundary. **Forbid whole-vector cosine in final decisions.**  
* Validate at startup that all boundaries conform to the current schema version.

**“Publish vocabularies, units, and scaling rules”**

When I say this, I mean: **define exactly what each slice understands and how it measures things**, so boundaries and intents can’t drift apart.

* **Vocabularies:** the allowed words or enums for a slot.  
  * Example: the `action` slot only accepts `{read, write, delete, export}`.  
  * Both boundary and intent encoders must use the same list and internal IDs.  
* **Units:** how numeric values are measured.  
  * Example: risk scores are always 0–1, time is in hours (0–23), duration is in seconds.  
* **Scaling rules:** how you convert or normalize a value into the embedding.  
  * Example: take log(value \+ 1\) or divide by max\_value before projecting.

Publishing this means you make it explicit and versioned—so every agent, service, or data pipeline uses **the same dictionary and math** when encoding those fields.

**“Forbid whole-vector cosine in final decisions”**

If you take the cosine of the **entire 128-dimensional vector**, you mix unrelated meanings:

* 32 dimensions about “action” and 32 about “risk” will interact mathematically even though they describe different ideas.  
* A small change in the risk slice could make the overall cosine look higher even if the action or data slice clearly violates a boundary.

By forbidding the whole-vector cosine, we ensure each semantic slice (action, resource, data, risk) is compared **only to its counterpart**.  
Then the system combines those per-slice scores using clear rules (min or weighted average).  
This keeps results interpretable—if an intent fails, you can see *which slice* caused it—and prevents false approvals due to cross-slice blending.

**Mental Model**

Instead of comparing two vectors in a 128-dim embedding space directly, we think about these vectors as the sum of 4 other vectors where their order is preserved. We then compare each of these 4 vectors with their counterparts which constitute the other vector.

We are treating the **128-dimensional vector as four ordered sub-vectors**, each with its own meaning and dimensional range:

* Action (0–31)  
* Resource (32–63)  
* Data (64–95)  
* Risk (96–127)

Instead of taking one big cosine over the entire 128-dim vector, we:

1. **Slice** the two vectors into those four parts.  
2. **Compare each slice with its matching slice** from the other vector (e.g., action↔action).  
3. **Aggregate the four similarities** according to boundary rules (min for mandatory, weighted average for optional).

That’s what makes the comparison *semantically aligned* — it ensures each concept is only compared to its direct counterpart, not blended with unrelated dimensions.

# Canonicalization Without Token Sorting

**Purpose.** Preserve structural semantics of paths.

**Spec.**

* Keep original path order. Use dotted paths and numeric array indices.  
* Normalize case and punctuation only.  
* Example: `user.profile[0].email`.  
* Do not alphabetically sort tokens. If order must be ignored for a specific field, mark that field as `order_invariant: true` and use multiset hashing only for that field.

