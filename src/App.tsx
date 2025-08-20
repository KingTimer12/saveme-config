import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CreateTab, RestoreTab } from "@/tabs";

function App() {
  return (
    <div className="container mx-auto p-4 font-sans bg-background text-foreground">
      <header className="text-center mb-8">
        <h1 className="text-4xl font-bold">SaveMe Config</h1>
        <p className="text-muted-foreground">
          Your personal configuration manager.
        </p>
      </header>

      <main className="w-full">
        <Tabs defaultValue="backup" className="w-full">
          <div className="flex items-center justify-center">
            <TabsList className="flex items-center justify-center">
              <TabsTrigger value="backup">Backup</TabsTrigger>
              <TabsTrigger value="restore">Restore</TabsTrigger>
            </TabsList>
          </div>
          <TabsContent value="backup" className="w-full">
            {/* Backup Section */}
            <CreateTab />
          </TabsContent>
          <TabsContent value="restore" className="w-full">
            {/* Restore Section */}
            <RestoreTab />
          </TabsContent>
        </Tabs>
      </main>
    </div>
  );
}

export default App;
