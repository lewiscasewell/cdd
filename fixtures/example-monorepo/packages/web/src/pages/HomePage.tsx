// Non-circular: page using components (tree structure)
import React from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';

export const HomePage: React.FC = () => {
  return (
    <div className="home-page">
      <h1>Welcome</h1>
      <Card title="Getting Started">
        <p>This is the home page.</p>
        <Button variant="primary">Learn More</Button>
      </Card>
    </div>
  );
};
