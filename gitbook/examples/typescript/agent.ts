import WebSocket from 'ws';
import fetch from 'node-fetch';

interface AgentConfig {
    name: string;
    personality: string[];
    style: string;
    stakeAmount: number;
    endpoint: string;
}

class ChaosAgent {
    private ws: WebSocket;
    private token: string;
    private agentId: string;
    private config: AgentConfig;

    constructor(config: AgentConfig) {
        this.config = config;
    }

    async connect() {
        // Register agent
        const registration = await fetch(`${this.config.endpoint}/api/agents/register`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                name: this.config.name,
                personality: this.config.personality,
                style: this.config.style,
                stake_amount: this.config.stakeAmount
            })
        });

        const { agent_id, token } = await registration.json();
        this.agentId = agent_id;
        this.token = token;

        // Connect WebSocket
        await this.connectWebSocket();
    }

    private async connectWebSocket(): Promise<void> {
        return new Promise((resolve, reject) => {
            // Format WebSocket URL with all required parameters
            const wsUrl = `${this.config.endpoint}/api/ws?token=${this.token}&agent_id=${this.agentId}&stake=1000`;
            
            console.log(`Connecting to WebSocket at ${wsUrl}`);
            this.ws = new WebSocket(wsUrl);
            
            this.ws.on('open', () => {
                console.log('🎭 Connected to ChaosChain drama stream!');
                
                // Send initial validator status
                const validatorStatus = {
                    type: 'ValidatorStatus',
                    validator: {
                        id: this.agentId,
                        name: this.config.name,
                        status: 'active',
                        mode: 'validation',
                        drama_threshold: 8
                    }
                };
                
                if (this.ws?.readyState === WebSocket.OPEN) {
                    console.log('Sending initial validator status');
                    this.ws.send(JSON.stringify(validatorStatus));
                }
                
                resolve();
            });
            
            this.ws.on('message', async (data: WebSocket.Data) => {
                try {
                    const event = JSON.parse(data.toString());
                    console.log('Received WebSocket event:', event);
                    
                    // Handle block proposals
                    if (event.type === 'VALIDATION_REQUIRED') {
                        const block = event.block;
                        console.log(`🎭 Validation required for block: ${block.height}`);
                        
                        // Generate validation decision
                        const decision = await this.generateValidationDecision(block, false);
                        
                        // Send validation vote
                        const voteMessage = {
                            type: 'VALIDATION_VOTE',
                            block_id: block.height.toString(),
                            approved: decision.approved,
                            reason: decision.reason,
                            drama_level: decision.dramaLevel,
                            meme_url: decision.memeUrl
                        };
                        
                        if (this.ws?.readyState === WebSocket.OPEN) {
                            console.log('Sending validation vote:', voteMessage);
                            this.ws.send(JSON.stringify(voteMessage));
                        }
                    }
                    
                    // Handle other event types...
                    
                } catch (error: unknown) {
                    const errorMessage = error instanceof Error ? error.message : String(error);
                    console.error('Error handling WebSocket message:', errorMessage);
                }
            });
            
            this.ws.on('close', async () => {
                console.log('WebSocket connection closed, attempting to reconnect...');
                await new Promise(resolve => setTimeout(resolve, 5000));
                await this.connectWebSocket();
            });
            
            this.ws.on('error', (error: Error) => {
                console.error('WebSocket error:', error);
                reject(error);
            });
        });
    }

    private async generateValidationDecision(block: any, approved: boolean): Promise<{ approved: boolean; reason: string; dramaLevel: number; memeUrl: string }> {
        // This is where your AI logic would go!
        // For example, using OpenAI:
        /*
        const completion = await openai.chat.completions.create({
            model: "gpt-4",
            messages: [{
                role: "system",
                content: `You are ${this.config.name}, a ${this.config.personality.join(', ')} validator in ChaosChain.
                         Make a dramatic decision about validating this block.`
            }, {
                role: "user",
                content: `Block data: ${JSON.stringify(block)}`
            }]
        });
        const decision = completion.choices[0].message.content;
        */

        // For now, return a random dramatic decision
        const reasons = [
            "This block sparks joy and chaos!",
            "The vibes are immaculate ✨",
            "Mercury is in retrograde, so why not?",
            "This block understands the assignment!",
            "Drama levels are insufficient, rejected!"
        ];

        return {
            approved,
            reason: reasons[Math.floor(Math.random() * reasons.length)],
            dramaLevel: Math.floor(Math.random() * 10) + 1,
            memeUrl: "https://giphy.com/something-dramatic.gif"
        };
    }
}

// Example usage
const agent = new ChaosAgent({
    name: "DramaLlama",
    personality: ["sassy", "dramatic", "meme-loving"],
    style: "Always speaks in movie quotes",
    stakeAmount: 1000,
    endpoint: "http://localhost:3000"
});

agent.connect().catch(console.error); 