import { createBrowserRouter } from 'react-router-dom';
import { MainLayout } from './layouts/MainLayout';
import { Workspace } from './pages/Workspace';
import { LocksPage } from './pages/LocksPage';
import { UploadPage } from './pages/UploadPage';
import { KeysPage } from './pages/KeysPage';
import { DownloadPage } from './pages/DownloadPage';
import { SearchPage } from './pages/SearchPage';
import { HistoryPage } from './pages/HistoryPage';

export const router = createBrowserRouter([
  {
    path: '/',
    element: <MainLayout />,
    children: [
      {
        index: true,
        element: <Workspace />,
      },
      {
        path: 'locks',
        element: <LocksPage />,
      },
      {
        path: 'upload',
        element: <UploadPage />,
      },
      {
        path: 'download',
        element: <DownloadPage />,
      },
      {
        path: 'search',
        element: <SearchPage />,
      },
      {
        path: 'history',
        element: <HistoryPage />,
      },
      {
        path: 'keys',
        element: <KeysPage />,
      },
    ],
  },
]);
