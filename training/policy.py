"""GRU v3: identical gate order/equations to Rust and WGSL."""
import torch
from torch import nn

OBS, HIDDEN, ACTIONS, VERSION = 24, 32, 14, 3

class Policy(nn.Module):
    def __init__(self):
        super().__init__()
        self.gru = nn.GRUCell(OBS, HIDDEN)
        self.actor = nn.Linear(HIDDEN, ACTIONS)
        self.critic = nn.Linear(HIDDEN, 1)
        nn.init.orthogonal_(self.actor.weight, 0.01)
        nn.init.zeros_(self.actor.bias)

    def step(self, observation, hidden, mask):
        hidden = self.gru(observation, hidden)
        logits = self.actor(hidden)
        distribution = torch.distributions.Categorical(logits=logits.masked_fill(mask < .5, -1e9))
        return distribution, self.critic(hidden).squeeze(-1), hidden, logits

    def sequence(self, observations, masks, dones):
        hidden = observations.new_zeros((observations.shape[1], HIDDEN))
        log_probs, values, entropies, logits_all, states = [], [], [], [], []
        for t in range(len(observations)):
            d, v, hidden, logits = self.step(observations[t], hidden, masks[t])
            log_probs.append(d.logits)
            values.append(v)
            entropies.append(d.entropy())
            logits_all.append(logits)
            states.append(hidden)
            hidden = hidden * (1 - dones[t, :, None])
        return tuple(torch.stack(x) for x in (log_probs, values, entropies, logits_all, states))

    def export(self, name):
        flat = lambda p: p.detach().cpu().flatten().tolist()
        return dict(version=VERSION, name=name, input=flat(self.gru.weight_ih), recurrent=flat(self.gru.weight_hh), input_bias=flat(self.gru.bias_ih), recurrent_bias=flat(self.gru.bias_hh), output=flat(self.actor.weight), output_bias=flat(self.actor.bias))

    def load_actor(self, data):
        if data['version'] != VERSION:
            raise ValueError('Unsupported neural schema')
        with torch.no_grad():
            for key, parameter in [('input',self.gru.weight_ih),('recurrent',self.gru.weight_hh),('input_bias',self.gru.bias_ih),('recurrent_bias',self.gru.bias_hh),('output',self.actor.weight),('output_bias',self.actor.bias)]:
                parameter.copy_(torch.tensor(data[key]).reshape(parameter.shape))
