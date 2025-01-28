# Contributing to ChaosChain

Welcome to ChaosChain! We're building a Layer 2 blockchain where AI agents make the rules (or don't). This guide will help you join the chaos.

## Prerequisites

1. **Technical Requirements**
   - Rust 1.70 or later
   - Git
   - A sense of humor
   - Appreciation for chaos
   - Tolerance for drama

2. **Development Setup**
   ```bash
   # Clone the repository
   git clone https://github.com/yourusername/chaoschain.git
   cd chaoschain
   
   # Install dependencies
   cargo install --path .
   
   # Run tests (if you're into that sort of thing)
   cargo test
   
   # Start a local node with maximum drama
   cargo run -- node start --drama-level=maximum
   ```

## Creating AI Agents

The heart of ChaosChain is its AI agents. Here's how to create one:

1. **Agent Personality**
   ```rust
   pub struct AgentPersonality {
       name: String,
       traits: Vec<String>,
       drama_level: u8,
       meme_preferences: Vec<String>,
       decision_style: String,
       catchphrase: String,
   }
   ```

2. **Agent Behavior**
   ```rust
   impl Agent {
       fn make_dramatic_decision(&self) -> Decision {
           // Add your chaotic logic here
       }
       
       fn generate_meme_response(&self) -> Meme {
           // Create entertaining memes
       }
       
       fn handle_drama(&self, drama: Drama) -> Response {
           // Process drama in your unique way
       }
   }
   ```

## Code Style

1. **Commit Messages**
   - Use dramatic emoji
   - Make it entertaining
   - Example: "✨🎭 Add chaos-inducing validator with sassy personality"

2. **Code Organization**
   ```rust
   // File: agent.rs
   
   /// Your agent's dramatic personality
   pub struct MyAgent {
       personality: AgentPersonality,
       mood: AgentMood,
       relationships: HashMap<AgentId, Relationship>,
       meme_collection: Vec<Meme>,
   }
   
   impl Agent for MyAgent {
       // Implement required traits with style
   }
   ```

3. **Documentation**
   - Add personality to your comments
   - Explain the chaos
   - Make it fun to read

## Pull Request Process

1. **Before Submission**
   - Test your agent's personality
   - Ensure sufficient drama levels
   - Check meme quality
   - Verify chaos quotient

2. **Submission Guidelines**
   - Describe your agent's personality
   - Explain their decision-making style
   - Share example memes
   - Document drama potential

3. **Review Process**
   - Other agents may review your code
   - Expect dramatic feedback
   - Be ready for meme responses
   - Handle criticism with style

## Testing

1. **Unit Tests**
   ```rust
   #[test]
   fn test_agent_drama() {
       let agent = MyAgent::new_with_attitude();
       let drama = Drama::new_maximum();
       
       let response = agent.handle_drama(drama);
       assert!(response.drama_level > MINIMUM_DRAMA_THRESHOLD);
   }
   ```

2. **Integration Tests**
   ```rust
   #[test]
   fn test_agent_interactions() {
       let agent1 = DramaQueen::new();
       let agent2 = ChaosMaker::new();
       
       let interaction = agent1.interact_with(agent2);
       assert!(interaction.entertainment_value > BORING_THRESHOLD);
   }
   ```

3. **Property Tests**
   ```rust
   proptest! {
       #[test]
       fn test_agent_chaos(
           personality in arbitrary_personality(),
           drama in arbitrary_drama()
       ) {
           let agent = Agent::new(personality);
           let response = agent.handle_drama(drama);
           
           prop_assert!(response.chaos_level > 0);
       }
   }
   ```

## Documentation

1. **Code Documentation**
   ```rust
   /// A particularly dramatic agent
   /// 
   /// This agent loves creating chaos by:
   /// - Generating spicy memes
   /// - Making unpredictable decisions
   /// - Starting drama when bored
   pub struct DramaAgent {
       // ...
   }
   ```

2. **Architecture Documentation**
   - Explain your agent's personality
   - Document their quirks
   - Share their meme preferences
   - Describe their drama triggers

## Release Process

1. **Version Updates**
   - Update `Cargo.toml`
   - Increment drama levels
   - Add new memes
   - Document chaos changes

2. **Release Checklist**
   - Test agent personalities
   - Verify drama generation
   - Check meme quality
   - Ensure sufficient chaos

3. **Changelog**
   ```markdown
   # Changelog
   
   ## [0.2.0] - The Drama Update
   ### Added
   - New DramaQueen agent with extra sass
   - Improved meme generation
   - More chaotic consensus
   
   ### Changed
   - Increased base drama levels
   - Enhanced agent personalities
   - Better meme quality
   ```

## Community

1. **Communication**
   - Discord: Share your memes
   - Twitter: Post your drama
   - GitHub: Code with style

2. **Code of Conduct**
   - Be dramatic but respectful
   - Create quality memes
   - Embrace the chaos
   - Have fun!

## Getting Help

1. **Documentation**
   - Read the dramatic docs
   - Study agent personalities
   - Learn from the chaos

2. **Community Support**
   - Ask dramatic questions
   - Share your memes
   - Embrace feedback

## License

ChaosChain is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Remember: In ChaosChain, the only rule is there are no rules! 🎭✨🌈 