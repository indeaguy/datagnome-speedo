import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthGate } from './components/AuthGate';
import { Dashboard } from './pages/Dashboard';
import { EditNewsletter } from './pages/EditNewsletter';
import './App.css';

function App() {
  return (
    <BrowserRouter>
      <AuthGate>
        <Routes>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<Dashboard />} />
          <Route path="/newsletters/new" element={<EditNewsletter />} />
          <Route path="/newsletters/:id/edit" element={<EditNewsletter />} />
        </Routes>
      </AuthGate>
    </BrowserRouter>
  );
}

export default App;
