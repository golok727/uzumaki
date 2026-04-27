import { useState } from 'react';
import { C } from './theme';
import type { Tab } from './types';
import { Sidebar } from './sidebar';
import { Modal } from './modal';
import { DashboardPage } from './pages/dashboardPage';
import { InputsPage } from './pages/inputsPage';
import { LayoutPage } from './pages/layoutPage';
import { StressPage } from './pages/stressPage';
import { EventsPage } from './pages/eventsPage';
import { IssuesPage } from './pages/issuesPage';
import { ImagesPage } from './pages/imagesPage';
import { TimerPage } from './pages/timerPage';

export function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [showModal, setShowModal] = useState(false);

  const page = {
    dashboard: <DashboardPage />,
    inputs: <InputsPage />,
    layout: <LayoutPage />,
    stress: <StressPage />,
    events: <EventsPage />,
    issues: <IssuesPage />,
    images: <ImagesPage />,
    timer: <TimerPage />,
  }[activeTab];

  return (
    <view
      display="flex"
      flexDir="row"
      w="full"
      h="full"
      bg={C.bg}
      position="relative"
    >
      <Sidebar
        w="16%"
        active={activeTab}
        setActive={setActiveTab}
        onOpenModal={() => setShowModal(true)}
      />
      <view w="84%" h="full" display="flex" flexDir="col" bg={C.bg}>
        {page}
      </view>
      {showModal && <Modal onClose={() => setShowModal(false)} />}
    </view>
  );
}
